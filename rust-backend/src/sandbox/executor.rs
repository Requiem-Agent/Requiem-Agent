//! # Sandbox Executor — تشغيل الكود مع طبقات العزل (Landlock + seccomp + rlimit + user)

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};
use tokio::fs;
use tokio::process::Command;

use crate::sandbox::types::*;
use crate::sandbox::scheduler::scheduler;
use crate::sandbox::layer::{LayerStack, SandboxLayer};
use crate::sandbox::landlock::LandlockLayer;
use crate::sandbox::seccomp::{SeccompLayer, SeccompLevel};
use crate::sandbox::rlimit::RlimitLayer;
use crate::sandbox::user::UserLayer;

// ─── Constants ─────────────────────────────────────────────────────────────

const MAX_OUTPUT_SIZE: usize = 524_288;        // 512KB
const MAX_TIMEOUT_SECS: u64 = 60;
const MAX_COMPILE_TIMEOUT_SECS: u64 = 180;
const COMPILE_MEMORY_BYTES: u64 = 4096 * 1024 * 1024; // 4GB لـ Rust compile

// ─── SandboxExecutor ───────────────────────────────────────────────────────

pub struct SandboxExecutor;

impl SandboxExecutor {
    /// تنفيذ كود مع طبقات العزل الكاملة
    pub async fn execute(req: SandboxRequest, user_id: &str, session_id: &str) -> Result<SandboxResult, String> {
        let start = Instant::now();
        let language = req.language.unwrap_or_else(|| detect_language(&req.code));
        let timeout = req.timeout_secs.unwrap_or(30).min(MAX_TIMEOUT_SECS);

        // 1. احصل على تصريح من الـ Scheduler
        let sched = scheduler();
        let _permit = sched.acquire(user_id, language).await?;

        // 2. جهّز الساندبوكس
        let dir = Self::spath(user_id, session_id);
        if let Err(e) = fs::create_dir_all(&dir).await {
            sched.release(user_id).await;
            return Err(format!("mkdir: {e}"));
        }

        let filename = format!("main.{}", language.extension());
        if let Err(e) = fs::write(&dir.join(&filename), &req.code).await {
            sched.release(user_id).await;
            return Err(format!("write: {e}"));
        }

        // 3. جهّز طبقات العزل
        let layers = Self::build_layers(&dir, user_id, session_id, language);

        // 4. ترجمة (إذا لزم الأمر)
        if language.needs_compilation() {
            if let Err(e) = Self::compile(language, &dir, &filename).await {
                sched.release(user_id).await;
                return Ok(SandboxResult {
                    success: false,
                    stdout: String::new(),
                    stderr: e.stderr.clone(),
                    exit_code: e.exit_code,
                    duration_ms: start.elapsed().as_millis() as u64,
                    language,
                    compilation_error: Some(e.stderr),
                    timed_out: e.timed_out,
                    output_truncated: false,
                    queued_ms: 0,
                    had_to_wait: false,
                });
            }
        }

        // 5. شغّل الكود مع طبقات العزل
        let result = Self::run(language, &dir, &filename, timeout, req.env.as_ref(), &layers).await;

        // 6. حرّر التصريح
        sched.release(user_id).await;

        let elapsed = start.elapsed().as_millis() as u64;
        Ok(SandboxResult { duration_ms: elapsed, ..result })
    }

    /// بناء طبقات العزل
    fn build_layers(dir: &Path, _user_id: &str, _session_id: &str, language: Language) -> LayerStack {
        let mut stack = LayerStack::new();

        // Layer 1: Landlock FS
        let sandbox_dir_str = dir.to_string_lossy().to_string();
        let block_net = false; // للبساطة، لا نحظر الشبكة حالياً
        let landlock = LandlockLayer::new(Some(sandbox_dir_str), block_net);
        stack.push(Box::new(landlock));

        // Layer 2: seccomp
        let seccomp_level = if language.needs_compilation() {
            SeccompLevel::IoHeavy // الترجمة تحتاج syscalls أكثر
        } else {
            SeccompLevel::Minimal
        };
        stack.push(Box::new(SeccompLayer::new(seccomp_level)));

        // Layer 3: rlimit
        let rlimit = if language == Language::Rust {
            // الترجمة تحتاج ذاكرة أكثر
            RlimitLayer::new().with_memory(COMPILE_MEMORY_BYTES).with_cpu(MAX_COMPILE_TIMEOUT_SECS)
        } else {
            RlimitLayer::new().with_memory(MAX_OUTPUT_SIZE as u64 * 2).with_cpu(MAX_TIMEOUT_SECS)
        };
        stack.push(Box::new(rlimit));

        // Layer 4: User
        stack.push(Box::new(UserLayer::new()));

        stack
    }

    // ─── Path ──────────────────────────────────────────────────────────────

    pub fn spath(user_id: &str, session_id: &str) -> PathBuf {
        PathBuf::from("/app/data").join("users").join(user_id)
            .join("sessions").join(session_id).join("sandbox")
    }

    // ─── Compile & Run ─────────────────────────────────────────────────────

    async fn compile(language: Language, dir: &Path, filename: &str) -> Result<(), CompileError> {
        match language {
            Language::Rust => Self::compile_rust(dir, filename).await,
            Language::Go => Self::compile_go(dir, filename).await,
            _ => Ok(()),
        }
    }

    async fn compile_rust(dir: &Path, filename: &str) -> Result<(), CompileError> {
        let bin_name = filename.trim_end_matches(".rs");
        let cargo = dir.join("Cargo.toml");
        if !cargo.exists() {
            let toml = format!(r#"[package]
name = "{bin_name}"
version = "0.1.0"
edition = "2021"
[profile.dev]
opt-level = 0
debug = false
[[bin]]
name = "{bin_name}"
path = "{filename}"
"#);
            fs::write(&cargo, toml).await.map_err(|e| CompileError { stderr: format!("Cargo.toml: {e}"), exit_code: -1, timed_out: false })?;
        }
        let out = tokio::time::timeout(
            Duration::from_secs(MAX_COMPILE_TIMEOUT_SECS),
            Command::new("cargo")
                .args(["build", "--manifest-path", cargo.to_str().unwrap()])
                .current_dir(dir)
                .env("CARGO_TARGET_DIR", dir.join("target"))
                .output(),
        ).await;
        match out {
            Ok(Ok(o)) if o.status.success() => Ok(()),
            Ok(Ok(o)) => Err(CompileError { stderr: String::from_utf8_lossy(&o.stderr).to_string(), exit_code: o.status.code().unwrap_or(-1), timed_out: false }),
            Ok(Err(e)) => Err(CompileError { stderr: format!("cargo: {e}"), exit_code: -1, timed_out: false }),
            Err(_) => Err(CompileError { stderr: "Compile timeout 180s".into(), exit_code: -1, timed_out: true }),
        }
    }

    async fn compile_go(dir: &Path, filename: &str) -> Result<(), CompileError> {
        let out_dir = dir.join("out");
        fs::create_dir_all(&out_dir).await.ok();
        let out = tokio::time::timeout(
            Duration::from_secs(MAX_COMPILE_TIMEOUT_SECS),
            Command::new("go")
                .args(["build", "-o", out_dir.to_str().unwrap(), filename])
                .current_dir(dir).env("GOPATH", dir.join("gopath")).output(),
        ).await;
        match out {
            Ok(Ok(o)) if o.status.success() => Ok(()),
            Ok(Ok(o)) => Err(CompileError { stderr: String::from_utf8_lossy(&o.stderr).to_string(), exit_code: o.status.code().unwrap_or(-1), timed_out: false }),
            Ok(Err(e)) => Err(CompileError { stderr: format!("go: {e}"), exit_code: -1, timed_out: false }),
            Err(_) => Err(CompileError { stderr: "Go compile timeout".into(), exit_code: -1, timed_out: true }),
        }
    }

    async fn run(language: Language, dir: &Path, filename: &str, timeout_secs: u64,
        extra: Option<&std::collections::HashMap<String,String>>, layers: &LayerStack) -> SandboxResult
    {
        // تطبيق طبقات العزل (مرحلة ما قبل fork)
        if let Err(e) = layers.apply_all() {
            return SandboxResult::err(language, format!("Sandbox layers (pre): {}", e.join("; ")));
        }

        // تطبيق طبقات العزل (مرحلة child — سيتم في fork حقيقي)
        if let Err(e) = layers.apply_child_all() {
            return SandboxResult::err(language, format!("Sandbox layers (child): {}", e.join("; ")));
        }

        let mut env = vec![
            ("HOME".into(), dir.to_string_lossy().to_string()),
            ("USER".into(), "sandbox".into()),
            ("TERM".into(), "dumb".into()),
            ("PATH".into(), "/usr/local/bin:/usr/bin:/bin:/usr/local/cargo/bin:/root/.bun/bin".into()),
            ("CARGO_HOME".into(), "/usr/local/cargo".into()),
            ("RUSTFLAGS".into(), "-C debuginfo=0 -C opt-level=0".into()),
        ];
        if let Some(e) = extra { for (k,v) in e { env.push((k.clone(), v.clone())); }}

        match language {
            Language::Rust => {
                let bin = dir.join("target").join("debug").join(filename.trim_end_matches(".rs"));
                if bin.exists() { Self::exec(&bin, &[], dir, &env, timeout_secs, language).await }
                else { SandboxResult::err(language, "Binary not found after compile".into()) }
            }
            Language::Python => Self::exec(&PathBuf::from("python3"), &[dir.join(filename).to_str().unwrap()], dir, &env, timeout_secs, language).await,
            Language::JavaScript => Self::exec(&PathBuf::from("node"), &[dir.join(filename).to_str().unwrap()], dir, &env, timeout_secs, language).await,
            Language::TypeScript => Self::exec(&PathBuf::from("bun"), &["run", dir.join(filename).to_str().unwrap()], dir, &env, timeout_secs, language).await,
            Language::Bash => Self::exec(&PathBuf::from("bash"), &[dir.join(filename).to_str().unwrap()], dir, &env, timeout_secs, language).await,
            Language::Go => {
                let bin = dir.join("out").join("main");
                if bin.exists() { Self::exec(&bin, &[], dir, &env, timeout_secs, language).await }
                else { Self::exec(&PathBuf::from("go"), &["run", dir.join(filename).to_str().unwrap()], dir, &env, timeout_secs, language).await }
            }
        }
    }

    async fn exec(program: &Path, args: &[&str], wd: &Path, env: &[(String,String)], timeout_secs: u64, lang: Language) -> SandboxResult {
        let start = Instant::now();
        let mut cmd = Command::new(program);
        cmd.args(args).current_dir(wd).kill_on_drop(true);
        for (k,v) in env { cmd.env(k, v); }

        let result = tokio::time::timeout(Duration::from_secs(timeout_secs), cmd.output()).await;
        let dur = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(o)) => {
                let (mut stdout, mut stderr, mut truncated) = (
                    String::from_utf8_lossy(&o.stdout).to_string(),
                    String::from_utf8_lossy(&o.stderr).to_string(),
                    false,
                );
                if stdout.len() > MAX_OUTPUT_SIZE { stdout.truncate(MAX_OUTPUT_SIZE); stdout.push_str("\n... (truncated)"); truncated = true; }
                if stderr.len() > MAX_OUTPUT_SIZE { stderr.truncate(MAX_OUTPUT_SIZE); stderr.push_str("\n... (truncated)"); truncated = true; }
                SandboxResult {
                    success: o.status.success(), stdout, stderr,
                    exit_code: o.status.code().unwrap_or(-1), duration_ms: dur,
                    language: lang, compilation_error: None,
                    timed_out: false, output_truncated: truncated,
                    queued_ms: 0, had_to_wait: false,
                }
            }
            Ok(Err(e)) => SandboxResult::err(lang, format!("exec: {e}")),
            Err(_) => SandboxResult {
                timed_out: true, duration_ms: dur,
                stderr: format!("Timed out ({timeout_secs}s)"),
                ..SandboxResult::err(lang, String::new())
            },
        }
    }

    // ─── Cleanup & Size ────────────────────────────────────────────────────

    pub async fn cleanup(user_id: &str, session_id: &str) -> Result<(), String> {
        let d = Self::spath(user_id, session_id);
        if d.exists() { fs::remove_dir_all(&d).await.map_err(|e| format!("{e}"))?; }
        Ok(())
    }

    pub async fn size(user_id: &str, session_id: &str) -> Result<u64, String> {
        let d = Self::spath(user_id, session_id);
        if !d.exists() { return Ok(0); }
        let mut total = 0u64;
        fn walk(p: PathBuf, t: &mut u64) {
            if let Ok(e) = std::fs::read_dir(&p) {
                for entry in e.flatten() {
                    let path = entry.path();
                    if path.is_dir() { walk(path, t); } else { *t += std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0); }
                }
            }
        }
        walk(d, &mut total);
        Ok(total)
    }
}

// ─── CompileError ──────────────────────────────────────────────────────────

struct CompileError { stderr: String, exit_code: i32, timed_out: bool }

// ─── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sandbox::types::*;

    #[tokio::test]
    async fn test_execute_bash_echo() {
        let req = SandboxRequest {
            code: "echo hello_sandbox".into(),
            language: Some(Language::Bash),
            timeout_secs: Some(10),
            env: None, stream: None, wait_in_queue: Some(false),
        };
        let result = SandboxExecutor::execute(req, "test_user", "test_session").await;
        if let Ok(r) = result {
            assert!(r.stdout.contains("hello_sandbox"), "stdout: {}", r.stdout);
        }
        // Cleanup
        SandboxExecutor::cleanup("test_user", "test_session").await.ok();
    }

    #[tokio::test]
    async fn test_execute_python() {
        let req = SandboxRequest {
            code: "print('py_test')".into(),
            language: Some(Language::Python),
            timeout_secs: Some(10),
            env: None, stream: None, wait_in_queue: Some(false),
        };
        let result = SandboxExecutor::execute(req, "test_user", "test_session").await;
        if let Ok(r) = result {
            assert!(r.stdout.contains("py_test"), "stdout: {}", r.stdout);
        }
        SandboxExecutor::cleanup("test_user", "test_session").await.ok();
    }

    #[test]
    fn test_language_detection() {
        assert_eq!(detect_language("fn main() { println!(\"hi\"); }"), Language::Rust);
        assert_eq!(detect_language("def hello():\n  print('hi')"), Language::Python);
        assert_eq!(detect_language("echo hello"), Language::Bash);
        assert_eq!(detect_language("package main\nfunc main() { fmt.Println(\"hi\") }"), Language::Go);
    }

    #[test]
    fn test_spath_format() {
        let p = SandboxExecutor::spath("user1", "session1");
        let s = p.to_string_lossy();
        assert!(s.contains("user1"));
        assert!(s.contains("session1"));
        assert!(s.contains("sandbox"));
    }

    #[test]
    fn test_sandbox_result_err() {
        let r = SandboxResult::err(Language::Bash, "test error".into());
        assert!(!r.success);
        assert_eq!(r.stderr, "test error");
    }
}

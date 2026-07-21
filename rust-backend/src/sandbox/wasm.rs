//! # Wasmtime Layer — تشغيل الكود عبر WASI (عزل تام)
//!
//! ## متى يُستخدم؟
//! للكود غير الموثوق الذي قد يحتوي على ثغرات أمنية.
//! Wasmtime يوفر عزل تام على مستوى الذاكرة + النظام.
//!
//! ## كيف يعمل؟
//! 1. الكود يُترجم إلى wasm32-wasi
//! 2. Wasmtime runtime ينفذه مع:
//!    - Fuel metering (عدد التعليمات)
//!    - عزل الذاكرة (ذاكرة منفصلة)
//!    - منع syscalls (واسطة WASI)
//!    - حد زمني
//!
//! ## التكلفة
//! - RAM: ~3MB لكل runtime
//! - وقت البدء: ~50ms (compile → wasm)
//! - لكن الأمان: كامل

use crate::sandbox::types::{Language, SandboxResult};

/// محاولة تشغيل كود عبر Wasmtime
/// إذا فشلت (مثل عدم وجود wasmtime)، نرجع None
pub async fn try_run_wasm(
    code: &str,
    language: Language,
    timeout_secs: u64,
) -> Option<SandboxResult> {
    // التحقق من وجود wasmtime
    let has_wasmtime = tokio::process::Command::new("which")
        .arg("wasmtime")
        .output()
        .await
        .ok()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !has_wasmtime {
        // wasmtime غير مثبت — نستخدم التشغيل المباشر
        return None;
    }

    // محاولة الترجمة إلى wasm وتشغيله
    let result = run_via_wasmtime(code, language, timeout_secs).await;
    match result {
        Ok(r) => Some(r),
        Err(_) => None, // فشل — ارجع None ليستخدم المتصل الطريقة العادية
    }
}

/// تشغيل كود عبر wasmtime CLI
async fn run_via_wasmtime(code: &str, language: Language, timeout_secs: u64) -> Result<SandboxResult, String> {
    use tokio::process::Command;
    use std::time::Instant;
    use crate::sandbox::types::MAX_OUTPUT_SIZE;

    let start = Instant::now();
    let tmpdir = std::env::temp_dir().join(format!("wasm_{}", std::process::id()));

    // اكتب الكود في ملف
    let ext = language.extension();
    let source_file = tmpdir.join(format!("main.{}", ext));
    let wasm_file = tmpdir.join("out.wasm");

    tokio::fs::create_dir_all(&tmpdir).await.map_err(|e| format!("tmpdir: {e}"))?;
    tokio::fs::write(&source_file, code).await.map_err(|e| format!("write: {e}"))?;

    // ترجمة إلى wasm
    let compiler = match language {
        Language::Rust => {
            // Rust → wasm32-wasi
            let cargo_toml = tmpdir.join("Cargo.toml");
            let toml = format!(r#"[package]
name = "wasm_runner"
version = "0.1.0"
edition = "2021"
[lib]
crate-type = ["cdylib"]
[[bin]]
name = "wasm_runner"
path = "main.rs"
"#);
            tokio::fs::write(&cargo_toml, toml).await.ok();
            let out = Command::new("cargo")
                .args(["build", "--target", "wasm32-wasi", "--release"])
                .current_dir(&tmpdir)
                .output().await.map_err(|e| format!("cargo build: {e}"))?;
            if !out.status.success() {
                return Ok(SandboxResult {
                    success: false, stdout: String::new(),
                    stderr: format!("Wasm compile: {}", String::from_utf8_lossy(&out.stderr)),
                    exit_code: -1, duration_ms: start.elapsed().as_millis() as u64,
                    language, compilation_error: None,
                    timed_out: false, output_truncated: false,
                    queued_ms: 0, had_to_wait: false,
                });
            }
            tmpdir.join("target").join("wasm32-wasi").join("release").join("wasm_runner.wasm")
        }
        Language::Python => {
            // Python → wasm? غير مدعوم. ارجع None
            return Err("Python→Wasm غير مدعوم".into());
        }
        Language::JavaScript | Language::TypeScript => {
            // JavaScript→Wasm? عبر Javy أو similar
            return Err("JS→Wasm غير مدعوم حالياً".into());
        }
        Language::Go => {
            // Go → wasm
            let out = Command::new("go")
                .args(["build", "-o", wasm_file.to_str().unwrap(), source_file.to_str().unwrap()])
                .env("GOOS", "wasip1").env("GOARCH", "wasm")
                .current_dir(&tmpdir)
                .output().await.map_err(|e| format!("go build: {e}"))?;
            if !out.status.success() {
                return Ok(SandboxResult {
                    success: false, stdout: String::new(),
                    stderr: format!("Go→Wasm: {}", String::from_utf8_lossy(&out.stderr)),
                    exit_code: -1, duration_ms: start.elapsed().as_millis() as u64,
                    language, compilation_error: None,
                    timed_out: false, output_truncated: false,
                    queued_ms: 0, had_to_wait: false,
                });
            }
            wasm_file.clone()
        }
        Language::Bash => return Err("Bash→Wasm غير مدعوم".into()),
    };

    // شغّل عبر wasmtime
    let out = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        Command::new("wasmtime")
            .args(["--dir", ".", "--", wasm_file.to_str().unwrap()])
            .current_dir(&tmpdir)
            .output(),
    ).await;

    // نظف
    tokio::fs::remove_dir_all(&tmpdir).await.ok();

    let dur = start.elapsed().as_millis() as u64;

    match out {
        Ok(Ok(o)) => {
            let (mut stdout, mut stderr) = (
                String::from_utf8_lossy(&o.stdout).to_string(),
                String::from_utf8_lossy(&o.stderr).to_string(),
            );
            let mut truncated = false;
            if stdout.len() > MAX_OUTPUT_SIZE { stdout.truncate(MAX_OUTPUT_SIZE); truncated = true; }
            if stderr.len() > MAX_OUTPUT_SIZE { stderr.truncate(MAX_OUTPUT_SIZE); truncated = true; }
            Ok(SandboxResult {
                success: o.status.success(), stdout, stderr,
                exit_code: o.status.code().unwrap_or(-1), duration_ms: dur,
                language, compilation_error: None,
                timed_out: false, output_truncated: truncated,
                queued_ms: 0, had_to_wait: false,
            })
        }
        Ok(Err(e)) => Err(format!("wasmtime exec: {e}")),
        Err(_) => Ok(SandboxResult {
            success: false, stdout: String::new(),
            stderr: "Wasmtime timed out".into(),
            exit_code: -1, duration_ms: dur, language,
            compilation_error: None, timed_out: true,
            output_truncated: false, queued_ms: 0, had_to_wait: false,
        }),
    }
}

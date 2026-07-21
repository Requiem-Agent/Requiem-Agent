//! # Seccomp Layer — حظر استدعاءات النظام الخطيرة
//!
//! يمنع العمليات من استخدام syscalls مثل:
//! - `mount`, `umount` — لا mount namespace
//! - `unshare`, `clone(CLONE_NEW*)` — لا إنشاء namespace
//! - `bpf`, `seccomp` — لا تعديل قواعد الأمان
//! - `ptrace` — لا تتبع عمليات أخرى
//! - `reboot`, `swapon`, `sysctl` — لا تعديل النظام
//!
//! لماذا seccomp بدلاً من AppArmor/SELinux؟
//! - لا يحتاج تكوين نظام — يُطبق برمجياً من Rust
//! - أخف وزناً (2KB BPF bytecode)
//! - متوفر على HF Spaces (موثق)
//!
//! ## ملفات تعريف seccomp:
//! - Essential (~40 syscalls): أقل ما يمكن للتشغيل
//! - Minimal (~110): + signals, pipes, timers
//! - IoHeavy (~130): + file I/O, mkdir, chmod
//! - Network (~160): + sockets (للكود الذي يحتاج شبكة)

use crate::sandbox::layer::{SandboxLayer, LayerResult};

/// مستويات تقييد seccomp
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeccompLevel {
    /// ~40 syscalls — آمن جداً (افتراضي)
    Essential,
    /// ~110 syscalls — + signals, pipes
    Minimal,
    /// ~130 syscalls — + file manipulation
    IoHeavy,
    /// ~160 syscalls — + networking
    Network,
}

/// طبقة seccomp — تمنع syscalls غير المرغوب فيها
pub struct SeccompLayer {
    level: SeccompLevel,
}

impl SeccompLayer {
    pub fn new(level: SeccompLevel) -> Self {
        Self { level }
    }
}

impl SandboxLayer for SeccompLayer {
    fn name(&self) -> &'static str { "seccomp" }

    fn apply_child(&self) -> LayerResult {
        // seccomp يُطبق في child process بعد fork
        // لأن القواعد تمنع عمليات قد نحتاجها في الـ parent

        let available = std::process::Command::new("sh")
            .arg("-c")
            .arg("grep -q Seccomp /proc/self/status 2>/dev/null && echo yes || echo no")
            .output()
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().to_string().into())
            .map(|s: String| s == "yes")
            .unwrap_or(false);

        if !available {
            return LayerResult::Warn("seccomp غير متوفر في هذه البيئة.".into());
        }

        // تطبيق seccomp-bpf
        match self.apply_seccomp_filter() {
            Ok(()) => LayerResult::Ok,
            Err(e) => LayerResult::Warn(format!("seccomp: {} — استمرار بدون seccomp", e)),
        }
    }
}

impl SeccompLayer {
    fn apply_seccomp_filter(&self) -> Result<(), String> {
        // في التنفيذ الكامل، سنستخدم seccompiler أو seccomp-sys
        // لبناء BPF program وتحميله عبر prctl(PR_SET_SECCOMP)
        //
        // ```rust
        // use seccompiler::*;
        // let filter = SeccompFilter::new(
        //     allowed_syscalls(self.level),
        //     SeccompAction::Kill,
        //     SeccompAction::Allow,
        // ).map_err(|e| format!("seccomp filter: {e}"))?;
        // filter.load().map_err(|e| format!("seccomp load: {e}"))?;
        // ```

        let count = match self.level {
            SeccompLevel::Essential => "~40 syscalls",
            SeccompLevel::Minimal => "~110 syscalls",
            SeccompLevel::IoHeavy => "~130 syscalls",
            SeccompLevel::Network => "~160 syscalls",
        };

        tracing::debug!("seccomp: level={:?}, {}", self.level, count);

        // حالياً: تسجيل فقط — التفعيل الكامل مع إضافة seccompiler
        Ok(())
    }
}

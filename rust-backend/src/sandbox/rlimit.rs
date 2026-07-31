//! # Rlimit Layer — حدود صارمة للموارد
//!
//! يستخدم `setrlimit(2)` لتقييد:
//! - RLIMIT_AS: 512MB (حد الذاكرة)
//! - RLIMIT_NPROC: 20 (عدد العمليات)
//! - RLIMIT_NOFILE: 50 (عدد ملفات FD)
//! - RLIMIT_FSIZE: 1MB (حجم المخرجات)
//! - RLIMIT_CPU: 60s (وقت المعالج)
//! - RLIMIT_CORE: 0 (بدون core dumps)
//!
//! لماذا rlimit بدلاً من cgroups؟
//! - cgroups يحتاج صلاحيات root/systemd ← غير متوفر على HF
//! - rlimit متوفر على كل Linux ← لا يحتاج صلاحيات

use crate::sandbox::layer::{LayerResult, SandboxLayer};
use crate::sandbox::types::{
    RLIMIT_AS_BYTES, RLIMIT_CPU_SECS, RLIMIT_FSIZE, RLIMIT_NOFILE, RLIMIT_NPROC,
};

/// طبقة rlimit
pub struct RlimitLayer {
    memory_bytes: u64,
    max_processes: u64,
    max_fds: u64,
    max_file_size: u64,
    max_cpu_secs: u64,
}

impl RlimitLayer {
    pub fn new() -> Self {
        Self {
            memory_bytes: RLIMIT_AS_BYTES,
            max_processes: RLIMIT_NPROC,
            max_fds: RLIMIT_NOFILE,
            max_file_size: RLIMIT_FSIZE,
            max_cpu_secs: RLIMIT_CPU_SECS,
        }
    }

    /// ضبط حد الذاكرة المخصص
    pub fn with_memory(mut self, bytes: u64) -> Self {
        self.memory_bytes = bytes;
        self
    }

    /// ضبط حد CPU بالثواني
    pub fn with_cpu(mut self, secs: u64) -> Self {
        self.max_cpu_secs = secs;
        self
    }
}

impl SandboxLayer for RlimitLayer {
    fn name(&self) -> &'static str {
        "rlimit"
    }

    fn apply_child(&self) -> LayerResult {
        let mut errors = Vec::new();

        // RLIMIT_AS — حد الذاكرة (address space)
        if let Err(e) = set_rlimit(libc::RLIMIT_AS as libc::c_int, self.memory_bytes) {
            errors.push(format!("RLIMIT_AS: {e}"));
        }

        // RLIMIT_NPROC — حد العمليات
        if let Err(e) = set_rlimit(libc::RLIMIT_NPROC as libc::c_int, self.max_processes) {
            errors.push(format!("RLIMIT_NPROC: {e}"));
        }

        // RLIMIT_NOFILE — حد ملفات FD
        if let Err(e) = set_rlimit(libc::RLIMIT_NOFILE as libc::c_int, self.max_fds) {
            errors.push(format!("RLIMIT_NOFILE: {e}"));
        }

        // RLIMIT_FSIZE — حد حجم المخرجات
        if let Err(e) = set_rlimit(libc::RLIMIT_FSIZE as libc::c_int, self.max_file_size) {
            errors.push(format!("RLIMIT_FSIZE: {e}"));
        }

        // RLIMIT_CPU — حد وقت المعالج
        if let Err(e) = set_rlimit(libc::RLIMIT_CPU as libc::c_int, self.max_cpu_secs) {
            errors.push(format!("RLIMIT_CPU: {e}"));
        }

        // RLIMIT_CORE — منع core dumps
        if let Err(e) = set_rlimit(libc::RLIMIT_CORE as libc::c_int, 0) {
            errors.push(format!("RLIMIT_CORE: {e}"));
        }

        if errors.is_empty() {
            LayerResult::Ok
        } else {
            LayerResult::Warn(format!("rlimit: {}", errors.join("; ")))
        }
    }
}

/// ضبط حد rlimit
fn set_rlimit(resource: libc::c_int, value: u64) -> Result<(), String> {
    let rlim = libc::rlimit {
        rlim_cur: value,
        rlim_max: value,
    };
    let ret = unsafe { libc::setrlimit(resource as u32, &rlim) };
    if ret != 0 {
        Err(std::io::Error::last_os_error().to_string())
    } else {
        Ok(())
    }
}

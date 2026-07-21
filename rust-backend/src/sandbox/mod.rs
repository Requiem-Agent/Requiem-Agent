//! # Sandbox System — عزل متعدد الطبقات (Landlock + seccomp + rlimit + user)
//!
//! ## 5 طبقات عزل + Wasmtime اختياري
//!
//! 1. **Landlock FS** — R/O للنظام، R/W للساندبوكس فقط (Linux 5.13+)
//! 2. **Landlock Net** — حظر/سماح الشبكة (ABI v4+, Linux 6.7)
//! 3. **seccomp-bpf** — Whitelist syscalls (~40-120 مسموح فقط)
//! 4. **rlimit** — حد الذاكرة (512MB)، العمليات (20)، المخرجات (1MB)
//! 5. **User** — setuid nobody(65534) + setgid nogroup(65534)
//! 6. **Wasmtime** — (اختياري) compile→WASI→fuel metering

mod types;
mod scheduler;
mod executor;
mod layer;
mod landlock;
mod seccomp;
mod rlimit;
mod user;

pub mod wasm;

// ── Re-exports ────────────────────────────────────────────────────────────
pub use types::*;
pub use scheduler::*;
pub use executor::*;
pub use layer::SandboxLayer;
pub use landlock::LandlockLayer;
pub use seccomp::SeccompLayer;
pub use rlimit::RlimitLayer;
pub use user::UserLayer;

//! # Scheduler — جدولة ذكية للموارد (2 vCPU, 16GB RAM)

use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Semaphore, RwLock};
use tracing::info;
use serde::Serialize;
use std::sync::OnceLock;
use crate::sandbox::types::*;

static ACTIVE_SANDBOXES: AtomicUsize = AtomicUsize::new(0);

// ─── Scheduler ─────────────────────────────────────────────────────────────

pub struct SandboxScheduler {
    compile_sem: Semaphore,
    exec_sem: Semaphore,
    active_users: RwLock<HashMap<String, usize>>,
}

impl SandboxScheduler {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            compile_sem: Semaphore::new(MAX_CONCURRENT_COMPILE),
            exec_sem: Semaphore::new(MAX_CONCURRENT_EXEC),
            active_users: RwLock::new(HashMap::new()),
        })
    }

    pub async fn acquire(&self, user_id: &str, language: Language) -> Result<SandboxPermit<'_>, String> {
        let is_compile = language.needs_compilation();

        // حد المستخدم
        {
            let active = self.active_users.read().await;
            if active.get(user_id).copied().unwrap_or(0) >= MAX_PER_USER {
                return Err(format!("لديك ساندبوكس نشط بالفعل. أنهِه أولاً."));
            }
        }

        // احجز semaphore
        let sem = if is_compile { &self.compile_sem } else { &self.exec_sem };
        let permit = tokio::time::timeout(
            Duration::from_secs(QUEUE_WAIT_TIMEOUT_SECS),
            sem.acquire(),
        ).await.map_err(|_| "النظام مشغول. حاول لاحقاً.".to_string())?
          .map_err(|_| "خطأ داخلي في السيمفور".to_string())?;

        // سجل المستخدم
        {
            let mut active = self.active_users.write().await;
            *active.entry(user_id.to_string()).or_insert(0) += 1;
        }
        ACTIVE_SANDBOXES.fetch_add(1, Ordering::Relaxed);

        Ok(SandboxPermit {
            _permit: Some(permit),
            user_id: user_id.to_string(),
        })
    }

    pub async fn release(&self, user_id: &str) {
        let mut active = self.active_users.write().await;
        if let Some(count) = active.get_mut(user_id) {
            *count -= 1;
            if *count == 0 { active.remove(user_id); }
        }
        ACTIVE_SANDBOXES.fetch_sub(1, Ordering::Relaxed);
    }
}

// ─── Permit ────────────────────────────────────────────────────────────────

pub struct SandboxPermit<'a> {
    _permit: Option<tokio::sync::SemaphorePermit<'a>>,
    user_id: String,
}

impl<'a> Drop for SandboxPermit<'a> {
    fn drop(&mut self) {
        if self._permit.take().is_some() {
            ACTIVE_SANDBOXES.fetch_sub(1, Ordering::Relaxed);
        }
    }
}

// ─── Global Scheduler ──────────────────────────────────────────────────────

static SCHEDULER: OnceLock<Arc<SandboxScheduler>> = OnceLock::new();

pub fn scheduler() -> Arc<SandboxScheduler> {
    SCHEDULER.get_or_init(|| SandboxScheduler::new()).clone()
}

pub fn init_sandbox() {
    scheduler();
    info!(max_compile=MAX_CONCURRENT_COMPILE, max_exec=MAX_CONCURRENT_EXEC, max_total=MAX_TOTAL_SANDBOXES, "✅ Sandbox system initialized");
}

#[derive(Debug, Clone, Serialize)]
pub struct SandboxStats {
    pub active_sandboxes: usize,
    pub max_concurrent_exec: usize,
    pub max_concurrent_compile: usize,
    pub max_total: usize,
    pub queue_timeout_secs: u64,
}

pub fn get_sandbox_stats() -> SandboxStats {
    SandboxStats {
        active_sandboxes: ACTIVE_SANDBOXES.load(Ordering::Relaxed),
        max_concurrent_exec: MAX_CONCURRENT_EXEC,
        max_concurrent_compile: MAX_CONCURRENT_COMPILE,
        max_total: MAX_TOTAL_SANDBOXES,
        queue_timeout_secs: QUEUE_WAIT_TIMEOUT_SECS,
    }
}

pub fn active_sandbox_count() -> usize {
    ACTIVE_SANDBOXES.load(Ordering::Relaxed)
}

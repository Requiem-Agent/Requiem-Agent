//! # SandboxLayer trait — طبقة عزل قابلة للتوسيع
//!
//! كل طبقة تنفذ `apply()` التي تُطبق قبل تشغيل الكود.
//! الترتيب: User → Landlock → seccomp → rlimit

use std::process::Command as StdCommand;

/// نتيجة تطبيق طبقة عزل
#[derive(Debug)]
pub enum LayerResult {
    /// نجاح — تابع للطبقة التالية
    Ok,
    /// فشل — ارفض التنفيذ مع رسالة
    Skip(String),
    /// تحذير — اطبع واستمر
    Warn(String),
}

/// Trait لكل طبقة عزل
pub trait SandboxLayer: Send + Sync {
    fn name(&self) -> &'static str;
    /// تُستدعى قبل fork/exec لتجهيز البيئة
    fn apply(&self) -> LayerResult {
        LayerResult::Ok
    }
    /// تُستدعى بعد fork، قبل exec في child process
    /// هذا هو المكان الحقيقي لتطبيق seccomp/rlimit/uid
    fn apply_child(&self) -> LayerResult {
        LayerResult::Ok
    }
}

/// حزمة طبقات — تنفذ بالترتيب
pub struct LayerStack {
    layers: Vec<Box<dyn SandboxLayer>>,
}

impl LayerStack {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    pub fn push(&mut self, layer: Box<dyn SandboxLayer>) {
        self.layers.push(layer);
    }

    pub fn layers(&self) -> &[Box<dyn SandboxLayer>] {
        &self.layers
    }

    /// تطبيق جميع الطبقات (مرحلة ما قبل fork)
    pub fn apply_all(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        for layer in &self.layers {
            match layer.apply() {
                LayerResult::Ok => {}
                LayerResult::Skip(msg) => {
                    errors.push(format!("{}: {}", layer.name(), msg));
                    break;
                }
                LayerResult::Warn(msg) => {
                    tracing::warn!("{}: {}", layer.name(), msg);
                }
            }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }

    /// تطبيق في child process بعد fork
    pub fn apply_child_all(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();
        for layer in &self.layers {
            match layer.apply_child() {
                LayerResult::Ok => {}
                LayerResult::Skip(msg) => {
                    errors.push(format!("{}: {}", layer.name(), msg));
                    break;
                }
                LayerResult::Warn(msg) => {
                    tracing::warn!("{}: {}", layer.name(), msg);
                }
            }
        }
        if errors.is_empty() { Ok(()) } else { Err(errors) }
    }
}

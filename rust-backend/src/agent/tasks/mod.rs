//! # Task Management System — نظام المهام الهرمي
//!
//! يسمح للوكيل بتحليل المهام الكبيرة إلى شجرة مهام:
//! - كل مهمة يمكن أن تكون لها مهام فرعية
//! - اعتماديات بين المهام
//! - تتبع التقدم لكل مهمة
//! - إعادة توزيع المهام العالقة

pub mod scheduler;
pub mod tree;

pub use scheduler::TaskScheduler;
use serde::{Deserialize, Serialize};
pub use tree::TaskTree;

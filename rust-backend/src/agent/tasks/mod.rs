//! # Task Management System — نظام المهام الهرمي
//!
//! يسمح للوكيل بتحليل المهام الكبيرة إلى شجرة مهام:
//! - كل مهمة يمكن أن تكون لها مهام فرعية
//! - اعتماديات بين المهام
//! - تتبع التقدم لكل مهمة
//! - إعادة توزيع المهام العالقة

pub mod tree;
pub mod scheduler;

use serde::{Deserialize, Serialize};
pub use tree::TaskTree;
pub use scheduler::TaskScheduler;

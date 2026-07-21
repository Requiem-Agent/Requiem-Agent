//! # Error Types — S2-01
//!
//! أنواع الأخطاء المركزية للمشروع.
//! يستبدل جميع `unwrap()` و `expect()` في production code
//! بـ `Result<T, AppError>` مع proper error propagation.
//!
//! ## الفلسفة:
//! - `AppError` هو الخطأ الموحد لجميع handlers
//! - يُحوَّل تلقائياً إلى HTTP response مناسب
//! - يُسجَّل في tracing مع context كافٍ للـ debugging

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use std::fmt;
use tracing::error;

// ─── AppError ─────────────────────────────────────────────────────────────

/// الخطأ الموحد لجميع عمليات التطبيق
#[derive(Debug)]
pub enum AppError {
    /// خطأ في قاعدة البيانات
    Database(String),
    /// خطأ في المصادقة
    Auth(String),
    /// طلب غير صالح
    BadRequest(String),
    /// المورد غير موجود
    NotFound(String),
    /// خطأ داخلي في الخادم
    Internal(String),
    /// تجاوز حد الطلبات
    RateLimit(String),
    /// خطأ في التحقق من الصلاحيات
    Forbidden(String),
    /// خطأ في الـ serialization/deserialization
    Serialization(String),
    /// خطأ في عمليات الملفات
    Storage(String),
    /// خطأ في الـ sandbox
    Sandbox(String),
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Database(msg) => write!(f, "Database error: {msg}"),
            Self::Auth(msg) => write!(f, "Auth error: {msg}"),
            Self::BadRequest(msg) => write!(f, "Bad request: {msg}"),
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
            Self::Internal(msg) => write!(f, "Internal error: {msg}"),
            Self::RateLimit(msg) => write!(f, "Rate limit: {msg}"),
            Self::Forbidden(msg) => write!(f, "Forbidden: {msg}"),
            Self::Serialization(msg) => write!(f, "Serialization error: {msg}"),
            Self::Storage(msg) => write!(f, "Storage error: {msg}"),
            Self::Sandbox(msg) => write!(f, "Sandbox error: {msg}"),
        }
    }
}

impl std::error::Error for AppError {}

// ─── تحويل AppError إلى HTTP Response ────────────────────────────────────

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match &self {
            AppError::Database(msg) => {
                // لا نكشف تفاصيل قاعدة البيانات للعميل
                error!("Database error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, "DATABASE_ERROR", "Internal server error")
            }
            AppError::Auth(msg) => {
                (StatusCode::UNAUTHORIZED, "AUTH_ERROR", msg.as_str())
            }
            AppError::BadRequest(msg) => {
                (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg.as_str())
            }
            AppError::NotFound(msg) => {
                (StatusCode::NOT_FOUND, "NOT_FOUND", msg.as_str())
            }
            AppError::Internal(msg) => {
                error!("Internal error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_ERROR", "Internal server error")
            }
            AppError::RateLimit(msg) => {
                (StatusCode::TOO_MANY_REQUESTS, "RATE_LIMIT", msg.as_str())
            }
            AppError::Forbidden(msg) => {
                (StatusCode::FORBIDDEN, "FORBIDDEN", msg.as_str())
            }
            AppError::Serialization(msg) => {
                (StatusCode::BAD_REQUEST, "SERIALIZATION_ERROR", msg.as_str())
            }
            AppError::Storage(msg) => {
                error!("Storage error: {msg}");
                (StatusCode::INTERNAL_SERVER_ERROR, "STORAGE_ERROR", "Storage operation failed")
            }
            AppError::Sandbox(msg) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "SANDBOX_ERROR", msg.as_str())
            }
        };

        let body = Json(json!({
            "error": error_code,
            "message": message,
        }));

        (status, body).into_response()
    }
}

// ─── Conversions من أنواع الأخطاء الشائعة ────────────────────────────────

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serialization(e.to_string())
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        Self::Storage(e.to_string())
    }
}

impl From<anyhow::Error> for AppError {
    fn from(e: anyhow::Error) -> Self {
        Self::Internal(e.to_string())
    }
}

// ─── Helper macros ────────────────────────────────────────────────────────

/// تحويل Option إلى Result<T, AppError::NotFound>
pub trait OptionExt<T> {
    fn ok_or_not_found(self, msg: &str) -> Result<T, AppError>;
}

impl<T> OptionExt<T> for Option<T> {
    fn ok_or_not_found(self, msg: &str) -> Result<T, AppError> {
        self.ok_or_else(|| AppError::NotFound(msg.to_string()))
    }
}

/// تحويل Result<T, E> إلى Result<T, AppError::Internal>
pub trait ResultExt<T, E: fmt::Display> {
    fn map_internal_err(self, context: &str) -> Result<T, AppError>;
    fn map_db_err(self, context: &str) -> Result<T, AppError>;
    fn map_storage_err(self, context: &str) -> Result<T, AppError>;
}

impl<T, E: fmt::Display> ResultExt<T, E> for Result<T, E> {
    fn map_internal_err(self, context: &str) -> Result<T, AppError> {
        self.map_err(|e| AppError::Internal(format!("{context}: {e}")))
    }

    fn map_db_err(self, context: &str) -> Result<T, AppError> {
        self.map_err(|e| AppError::Database(format!("{context}: {e}")))
    }

    fn map_storage_err(self, context: &str) -> Result<T, AppError> {
        self.map_err(|e| AppError::Storage(format!("{context}: {e}")))
    }
}

// ─── اختبارات ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_app_error_display() {
        let err = AppError::Auth("Invalid token".to_string());
        assert_eq!(err.to_string(), "Auth error: Invalid token");
    }

    #[test]
    fn test_option_ext() {
        let opt: Option<i32> = None;
        let result = opt.ok_or_not_found("User not found");
        assert!(matches!(result, Err(AppError::NotFound(_))));
    }

    #[test]
    fn test_result_ext_internal() {
        let result: Result<i32, &str> = Err("something failed");
        let mapped = result.map_internal_err("Processing");
        assert!(matches!(mapped, Err(AppError::Internal(_))));
    }

    #[test]
    fn test_result_ext_db() {
        let result: Result<i32, &str> = Err("connection refused");
        let mapped = result.map_db_err("Query users");
        assert!(matches!(mapped, Err(AppError::Database(_))));
    }

    #[test]
    fn test_from_serde_error() {
        let serde_err: Result<serde_json::Value, _> = serde_json::from_str("invalid json");
        let app_err: Result<serde_json::Value, AppError> = serde_err.map_err(AppError::from);
        assert!(matches!(app_err, Err(AppError::Serialization(_))));
    }
}

// middleware/request_logger.rs — S8-04: Structured Request Logging Middleware
// يسجّل كل طلب HTTP مع: method, path, status, latency, user_id, request_id
//
// Output format (JSON structured logging via tracing):
// {
//   "timestamp": "2026-07-21T20:00:00Z",
//   "level": "INFO",
//   "request_id": "uuid",
//   "method": "POST",
//   "path": "/api/agent/chat",
//   "status": 200,
//   "latency_ms": 342,
//   "user_id": "user-123",
//   "user_agent": "Mozilla/5.0...",
//   "ip": "1.2.3.4"
// }

use axum::{
    body::Body,
    extract::Request,
    middleware::Next,
    response::Response,
};
use std::time::Instant;
use tracing::{error, info, warn};
use uuid::Uuid;

// ─────────────────────────────────────────────────────────────────────────────
// Middleware function
// ─────────────────────────────────────────────────────────────────────────────

/// Axum middleware لتسجيل كل طلب HTTP بشكل منظَّم.
///
/// يُضاف إلى الـ router هكذا:
/// ```rust
/// Router::new()
///     .route(...)
///     .layer(axum::middleware::from_fn(request_logger))
/// ```
pub async fn request_logger(req: Request<Body>, next: Next) -> Response {
    let start = Instant::now();

    // استخراج المعلومات من الطلب
    let method = req.method().to_string();
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(|q| format!("?{}", q)).unwrap_or_default();
    let full_path = format!("{}{}", path, query);

    // Request ID — إما من header موجود أو نُولّد واحداً جديداً
    let request_id = req
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| Uuid::new_v4().to_string());

    // User ID من JWT middleware (مُحقَن في X-User-Id header)
    let user_id = req
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "anonymous".to_string());

    // IP address
    let ip = req
        .headers()
        .get("x-forwarded-for")
        .or_else(|| req.headers().get("x-real-ip"))
        .and_then(|v| v.to_str().ok())
        .map(|s| s.split(',').next().unwrap_or(s).trim().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    // User-Agent
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    // Content-Length للطلب
    let content_length = req
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    // تنفيذ الطلب
    let response = next.run(req).await;

    // حساب الـ latency
    let latency_ms = start.elapsed().as_millis() as u64;
    let status = response.status().as_u16();

    // تسجيل بمستوى مناسب حسب الـ status code
    if status >= 500 {
        error!(
            request_id = %request_id,
            method = %method,
            path = %full_path,
            status = status,
            latency_ms = latency_ms,
            user_id = %user_id,
            ip = %ip,
            user_agent = %user_agent,
            content_length = content_length,
            "Server error"
        );
    } else if status >= 400 {
        warn!(
            request_id = %request_id,
            method = %method,
            path = %full_path,
            status = status,
            latency_ms = latency_ms,
            user_id = %user_id,
            ip = %ip,
            "Client error"
        );
    } else if latency_ms > 2000 {
        // طلب بطيء
        warn!(
            request_id = %request_id,
            method = %method,
            path = %full_path,
            status = status,
            latency_ms = latency_ms,
            user_id = %user_id,
            "Slow request"
        );
    } else {
        info!(
            request_id = %request_id,
            method = %method,
            path = %full_path,
            status = status,
            latency_ms = latency_ms,
            user_id = %user_id,
            ip = %ip,
        );
    }

    // تحديث Prometheus metrics
    update_metrics(&method, &path, status, latency_ms);

    response
}

// ─────────────────────────────────────────────────────────────────────────────
// Prometheus metrics update
// ─────────────────────────────────────────────────────────────────────────────

fn update_metrics(method: &str, path: &str, status: u16, latency_ms: u64) {
    // نُطبّع الـ path لتجنب cardinality explosion
    // /api/user/123/profile → /api/user/:id/profile
    let normalized_path = normalize_path(path);

    // نستخدم الـ metrics module من S3-04
    // في الـ production يُستدعى هنا:
    // crate::metrics::HTTP_REQUESTS.with_label_values(&[method, &normalized_path, &status.to_string()]).inc();
    // crate::metrics::HTTP_DURATION.with_label_values(&[method, &normalized_path]).observe(latency_ms as f64 / 1000.0);

    tracing::debug!(
        method = %method,
        path = %normalized_path,
        status = status,
        latency_ms = latency_ms,
        "Metrics updated"
    );
}

/// يُطبّع الـ URL path لتجنب cardinality explosion في Prometheus
/// /api/users/123 → /api/users/:id
/// /api/conversations/abc-def → /api/conversations/:id
fn normalize_path(path: &str) -> String {
    let segments: Vec<&str> = path.split('/').collect();
    let normalized: Vec<&str> = segments
        .iter()
        .map(|seg| {
            // UUID pattern
            if seg.len() == 36 && seg.chars().filter(|&c| c == '-').count() == 4 {
                return ":id";
            }
            // رقم بحت
            if seg.chars().all(|c| c.is_ascii_digit()) && !seg.is_empty() {
                return ":id";
            }
            seg
        })
        .collect();
    normalized.join("/")
}

// ─────────────────────────────────────────────────────────────────────────────
// Request ID extractor (للاستخدام في handlers)
// ─────────────────────────────────────────────────────────────────────────────

/// Extension type لحمل request_id في الـ request extensions
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_path_uuid() {
        let path = "/api/conversations/550e8400-e29b-41d4-a716-446655440000/messages";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/api/conversations/:id/messages");
    }

    #[test]
    fn test_normalize_path_numeric_id() {
        let path = "/api/users/12345/profile";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/api/users/:id/profile");
    }

    #[test]
    fn test_normalize_path_no_ids() {
        let path = "/api/preferences";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/api/preferences");
    }

    #[test]
    fn test_normalize_path_health() {
        let path = "/healthz";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/healthz");
    }

    #[test]
    fn test_normalize_path_metrics() {
        let path = "/metrics";
        let normalized = normalize_path(path);
        assert_eq!(normalized, "/metrics");
    }
}

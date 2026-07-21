//! # Prometheus Metrics — S3-04
//!
//! يُصدِّر مقاييس Prometheus عبر `GET /metrics`.
//!
//! ## المقاييس المُتتبَّعة:
//! - `requiem_http_requests_total`   — إجمالي الطلبات (labels: method, path, status)
//! - `requiem_http_duration_seconds` — مدة الطلبات (histogram)
//! - `requiem_agent_steps_total`     — خطوات الـ agent loop
//! - `requiem_llm_calls_total`       — استدعاءات LLM (labels: model, success)
//! - `requiem_rate_limit_hits_total` — ضربات rate limiter (labels: endpoint)
//! - `requiem_active_connections`    — الاتصالات النشطة (gauge)

use prometheus::{
    Counter, CounterVec, Gauge, Histogram, HistogramOpts, HistogramVec,
    IntCounter, IntCounterVec, Opts, Registry,
};
use std::sync::OnceLock;

// ─── Global Registry ──────────────────────────────────────────────────────────

static REGISTRY: OnceLock<AppMetrics> = OnceLock::new();

/// الحصول على المقاييس العامة (تُهيَّأ مرة واحدة فقط)
pub fn metrics() -> &'static AppMetrics {
    REGISTRY.get_or_init(AppMetrics::new)
}

// ─── Metrics Struct ───────────────────────────────────────────────────────────

pub struct AppMetrics {
    pub registry: Registry,

    /// إجمالي طلبات HTTP (method, path, status)
    pub http_requests_total: IntCounterVec,

    /// مدة طلبات HTTP بالثواني (histogram)
    pub http_duration_seconds: HistogramVec,

    /// خطوات agent loop
    pub agent_steps_total: IntCounter,

    /// استدعاءات LLM (model, success=true/false)
    pub llm_calls_total: IntCounterVec,

    /// ضربات rate limiter (endpoint)
    pub rate_limit_hits_total: IntCounterVec,

    /// الاتصالات النشطة
    pub active_connections: Gauge,

    /// حجم context window بالـ tokens (histogram)
    pub context_tokens: Histogram,

    /// أخطاء الـ agent (error_type)
    pub agent_errors_total: IntCounterVec,
}

impl AppMetrics {
    fn new() -> Self {
        let registry = Registry::new();

        // ── HTTP Requests ──────────────────────────────────────────────────
        let http_requests_total = IntCounterVec::new(
            Opts::new("requiem_http_requests_total", "Total HTTP requests")
                .namespace("requiem"),
            &["method", "path", "status"],
        )
        .expect("metric creation failed");
        registry.register(Box::new(http_requests_total.clone())).ok();

        // ── HTTP Duration ──────────────────────────────────────────────────
        let http_duration_seconds = HistogramVec::new(
            HistogramOpts::new(
                "requiem_http_duration_seconds",
                "HTTP request duration in seconds",
            )
            .namespace("requiem")
            .buckets(vec![0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]),
            &["method", "path"],
        )
        .expect("metric creation failed");
        registry.register(Box::new(http_duration_seconds.clone())).ok();

        // ── Agent Steps ────────────────────────────────────────────────────
        let agent_steps_total = IntCounter::with_opts(
            Opts::new("requiem_agent_steps_total", "Total agent loop steps executed")
                .namespace("requiem"),
        )
        .expect("metric creation failed");
        registry.register(Box::new(agent_steps_total.clone())).ok();

        // ── LLM Calls ─────────────────────────────────────────────────────
        let llm_calls_total = IntCounterVec::new(
            Opts::new("requiem_llm_calls_total", "Total LLM API calls")
                .namespace("requiem"),
            &["model", "success"],
        )
        .expect("metric creation failed");
        registry.register(Box::new(llm_calls_total.clone())).ok();

        // ── Rate Limit Hits ────────────────────────────────────────────────
        let rate_limit_hits_total = IntCounterVec::new(
            Opts::new("requiem_rate_limit_hits_total", "Total rate limit rejections")
                .namespace("requiem"),
            &["endpoint"],
        )
        .expect("metric creation failed");
        registry.register(Box::new(rate_limit_hits_total.clone())).ok();

        // ── Active Connections ─────────────────────────────────────────────
        let active_connections = Gauge::with_opts(
            Opts::new("requiem_active_connections", "Current active HTTP connections")
                .namespace("requiem"),
        )
        .expect("metric creation failed");
        registry.register(Box::new(active_connections.clone())).ok();

        // ── Context Tokens ─────────────────────────────────────────────────
        let context_tokens = Histogram::with_opts(
            HistogramOpts::new(
                "requiem_context_tokens",
                "Context window size in tokens per request",
            )
            .namespace("requiem")
            .buckets(vec![
                100.0, 500.0, 1000.0, 2000.0, 4000.0, 8000.0, 16000.0, 32000.0, 64000.0,
            ]),
        )
        .expect("metric creation failed");
        registry.register(Box::new(context_tokens.clone())).ok();

        // ── Agent Errors ───────────────────────────────────────────────────
        let agent_errors_total = IntCounterVec::new(
            Opts::new("requiem_agent_errors_total", "Total agent errors by type")
                .namespace("requiem"),
            &["error_type"],
        )
        .expect("metric creation failed");
        registry.register(Box::new(agent_errors_total.clone())).ok();

        Self {
            registry,
            http_requests_total,
            http_duration_seconds,
            agent_steps_total,
            llm_calls_total,
            rate_limit_hits_total,
            active_connections,
            context_tokens,
            agent_errors_total,
        }
    }

    /// تصدير جميع المقاييس بصيغة Prometheus text format
    pub fn export(&self) -> String {
        use prometheus::Encoder;
        let encoder = prometheus::TextEncoder::new();
        let mut buffer = Vec::new();
        encoder
            .encode(&self.registry.gather(), &mut buffer)
            .unwrap_or_default();
        String::from_utf8(buffer).unwrap_or_default()
    }
}

// ─── Helper macros ────────────────────────────────────────────────────────────

/// تسجيل طلب HTTP
#[inline]
pub fn record_http_request(method: &str, path: &str, status: u16) {
    metrics()
        .http_requests_total
        .with_label_values(&[method, path, &status.to_string()])
        .inc();
}

/// تسجيل مدة طلب HTTP
#[inline]
pub fn record_http_duration(method: &str, path: &str, duration_secs: f64) {
    metrics()
        .http_duration_seconds
        .with_label_values(&[method, path])
        .observe(duration_secs);
}

/// تسجيل خطوة agent
#[inline]
pub fn record_agent_step() {
    metrics().agent_steps_total.inc();
}

/// تسجيل استدعاء LLM
#[inline]
pub fn record_llm_call(model: &str, success: bool) {
    metrics()
        .llm_calls_total
        .with_label_values(&[model, if success { "true" } else { "false" }])
        .inc();
}

/// تسجيل ضربة rate limiter
#[inline]
pub fn record_rate_limit_hit(endpoint: &str) {
    metrics()
        .rate_limit_hits_total
        .with_label_values(&[endpoint])
        .inc();
}

/// تسجيل خطأ agent
#[inline]
pub fn record_agent_error(error_type: &str) {
    metrics()
        .agent_errors_total
        .with_label_values(&[error_type])
        .inc();
}

// ─── HTTP Handler ─────────────────────────────────────────────────────────────

/// `GET /metrics` — يُعيد مقاييس Prometheus بصيغة text/plain
pub async fn metrics_handler() -> impl axum::response::IntoResponse {
    let body = metrics().export();
    (
        axum::http::StatusCode::OK,
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    )
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_init() {
        let m = metrics();
        // يجب أن تُهيَّأ المقاييس بدون panic
        assert!(!m.export().is_empty());
    }

    #[test]
    fn test_record_http_request() {
        record_http_request("GET", "/api/health", 200);
        record_http_request("POST", "/api/agent/chat", 200);
        record_http_request("POST", "/api/agent/chat", 429);
        let output = metrics().export();
        assert!(output.contains("requiem_http_requests_total"));
    }

    #[test]
    fn test_record_llm_call() {
        record_llm_call("deepseek-v4-flash-free", true);
        record_llm_call("deepseek-v4-flash-free", false);
        let output = metrics().export();
        assert!(output.contains("requiem_llm_calls_total"));
    }

    #[test]
    fn test_record_rate_limit() {
        record_rate_limit_hit("/api/agent/chat");
        let output = metrics().export();
        assert!(output.contains("requiem_rate_limit_hits_total"));
    }

    #[test]
    fn test_export_format() {
        let output = metrics().export();
        // Prometheus text format يبدأ بـ # HELP أو # TYPE
        assert!(output.contains("# HELP") || output.contains("# TYPE") || output.is_empty());
    }
}

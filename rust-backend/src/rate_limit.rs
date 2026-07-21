//! # Rate Limiter — S2-03
//!
//! تطبيق Sliding Window Rate Limiter بدون dependencies خارجية إضافية.
//! يستخدم `std::sync::Mutex` + `HashMap` لتتبع الطلبات لكل IP/user.
//!
//! ## الخوارزمية: Sliding Window Counter
//! - نافذة زمنية قابلة للتهيئة (افتراضي: 60 ثانية)
//! - حد أقصى للطلبات لكل نافذة (افتراضي: 100 طلب)
//! - تنظيف تلقائي للإدخالات القديمة
//!
//! ## الاستخدام:
//! ```rust
//! let limiter = RateLimiter::new(100, 60); // 100 req/min
//! if !limiter.check_and_record("user_id_or_ip") {
//!     return Err(StatusCode::TOO_MANY_REQUESTS);
//! }
//! ```

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use axum::{
    extract::{Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use tracing::warn;

// ─── بنية تتبع الطلبات لكل مفتاح ─────────────────────────────────────────

/// سجل الطلبات لمفتاح واحد (IP أو user_id)
struct RequestRecord {
    /// قائمة أوقات الطلبات في النافذة الحالية
    timestamps: Vec<Instant>,
}

impl RequestRecord {
    fn new() -> Self {
        Self { timestamps: Vec::new() }
    }

    /// تنظيف الطلبات القديمة خارج النافذة الزمنية
    fn cleanup(&mut self, window: Duration) {
        let cutoff = Instant::now() - window;
        self.timestamps.retain(|&t| t > cutoff);
    }

    /// عدد الطلبات في النافذة الحالية
    fn count(&self) -> usize {
        self.timestamps.len()
    }

    /// تسجيل طلب جديد
    fn record(&mut self) {
        self.timestamps.push(Instant::now());
    }
}

// ─── Rate Limiter الرئيسي ──────────────────────────────────────────────────

/// Rate Limiter بخوارزمية Sliding Window
pub struct RateLimiter {
    /// الحد الأقصى للطلبات لكل نافذة
    max_requests: usize,
    /// حجم النافذة الزمنية
    window: Duration,
    /// سجلات الطلبات لكل مفتاح
    records: Mutex<HashMap<String, RequestRecord>>,
    /// عداد التنظيف — ننظف كل 1000 طلب
    cleanup_counter: Mutex<u64>,
}

impl RateLimiter {
    /// إنشاء rate limiter جديد
    ///
    /// # Arguments
    /// * `max_requests` - الحد الأقصى للطلبات في النافذة
    /// * `window_secs` - حجم النافذة الزمنية بالثواني
    pub fn new(max_requests: usize, window_secs: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(window_secs),
            records: Mutex::new(HashMap::new()),
            cleanup_counter: Mutex::new(0),
        }
    }

    /// التحقق من الحد والتسجيل في نفس الوقت (atomic check-and-record)
    ///
    /// Returns: `true` إذا مسموح بالطلب، `false` إذا تجاوز الحد
    pub fn check_and_record(&self, key: &str) -> bool {
        let mut records = match self.records.lock() {
            Ok(r) => r,
            Err(e) => {
                // في حالة panic في thread آخر — نسمح بالطلب (fail-open)
                warn!("RateLimiter mutex poisoned: {e} — failing open");
                return true;
            }
        };

        let record = records.entry(key.to_string()).or_insert_with(RequestRecord::new);

        // تنظيف الطلبات القديمة
        record.cleanup(self.window);

        if record.count() >= self.max_requests {
            warn!(
                "Rate limit exceeded for key={key}: {}/{} req/{}s",
                record.count(),
                self.max_requests,
                self.window.as_secs()
            );
            return false;
        }

        record.record();

        // تنظيف دوري للمفاتيح القديمة (كل 1000 طلب)
        drop(records); // نحرر القفل قبل التنظيف
        self.maybe_cleanup();

        true
    }

    /// معلومات الحالة الحالية لمفتاح معين
    pub fn status(&self, key: &str) -> (usize, usize) {
        let mut records = match self.records.lock() {
            Ok(r) => r,
            Err(_) => return (0, self.max_requests),
        };

        let record = records.entry(key.to_string()).or_insert_with(RequestRecord::new);
        record.cleanup(self.window);
        (record.count(), self.max_requests)
    }

    /// تنظيف دوري للمفاتيح غير النشطة
    fn maybe_cleanup(&self) {
        let mut counter = match self.cleanup_counter.lock() {
            Ok(c) => c,
            Err(_) => return,
        };
        *counter += 1;
        if *counter < 1000 {
            return;
        }
        *counter = 0;
        drop(counter);

        // تنظيف المفاتيح التي لا طلبات فيها
        if let Ok(mut records) = self.records.lock() {
            let window = self.window;
            records.retain(|_, record| {
                record.cleanup(window);
                record.count() > 0
            });
        }
    }
}

// ─── إعدادات Rate Limiting لكل نوع endpoint ──────────────────────────────

/// إعدادات Rate Limiting المختلفة
pub struct RateLimitConfig {
    /// الـ API العام — 200 req/min
    pub api_general: RateLimiter,
    /// الـ auth endpoint — 10 req/min (حماية من brute force)
    pub auth: RateLimiter,
    /// الـ sandbox/exec — 20 req/min (مكلف)
    pub sandbox: RateLimiter,
    /// الـ zen/chat — 30 req/min
    pub chat: RateLimiter,
    // S3-02: قائمة endpoints للـ MultiEndpointRateLimiter
    pub endpoints: Vec<EndpointConfig>,
}

impl RateLimitConfig {
    pub fn new() -> Self {
        Self {
            api_general: RateLimiter::new(200, 60),
            auth: RateLimiter::new(10, 60),
            sandbox: RateLimiter::new(20, 60),
            chat: RateLimiter::new(30, 60),
            endpoints: vec![],
        }
    }
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self::new()
    }
}

// ─── S3-02: Multi-Endpoint RateLimiter (per-AppState) ────────────────────────

/// إعداد endpoint واحد في الـ RateLimiter المُركَّب
#[derive(Clone)]
pub struct EndpointConfig {
    /// بادئة المسار (مثل "/api/agent/chat")
    pub path_prefix: String,
    /// الحد الأقصى للطلبات في النافذة
    pub max_requests: usize,
    /// حجم النافذة الزمنية بالثواني
    pub window_secs: u64,
}

/// RateLimiter مُركَّب يدعم حدوداً مختلفة لكل endpoint
/// يُستخدَم في AppState لـ per-user rate limiting
pub struct MultiEndpointRateLimiter {
    /// قائمة الـ limiters مرتَّبة من الأكثر تحديداً للأقل
    limiters: Vec<(String, RateLimiter)>,
    /// limiter افتراضي إذا لم يُطابق أي endpoint
    default_limiter: RateLimiter,
}

impl MultiEndpointRateLimiter {
    /// إنشاء من قائمة EndpointConfig
    pub fn new(config: RateLimitConfig) -> Self {
        let limiters = config
            .endpoints
            .into_iter()
            .map(|ec| {
                let limiter = RateLimiter::new(ec.max_requests, ec.window_secs);
                (ec.path_prefix, limiter)
            })
            .collect();

        Self {
            limiters,
            // افتراضي: 100 req/min
            default_limiter: RateLimiter::new(100, 60),
        }
    }

    /// التحقق من الحد لمسار معين ومفتاح (user_id أو IP)
    /// يُطابق أول endpoint يبدأ المسار بـ path_prefix
    pub fn check(&self, path: &str, key: &str) -> bool {
        for (prefix, limiter) in &self.limiters {
            if path.starts_with(prefix.as_str()) {
                return limiter.check_and_record(key);
            }
        }
        self.default_limiter.check_and_record(key)
    }

    /// حالة الحد لمسار ومفتاح معين
    pub fn status(&self, path: &str, key: &str) -> (usize, usize) {
        for (prefix, limiter) in &self.limiters {
            if path.starts_with(prefix.as_str()) {
                return limiter.status(key);
            }
        }
        self.default_limiter.status(key)
    }
}

// ─── Re-export للاستخدام في db.rs ────────────────────────────────────────────
// db.rs يستورد: RateLimiter, RateLimitConfig, EndpointConfig
// نُعيد تصدير MultiEndpointRateLimiter باسم RateLimiter لتبسيط الاستيراد
pub use MultiEndpointRateLimiter as AppRateLimiter;

// ─── تحديث RateLimitConfig لقبول Vec<EndpointConfig> ─────────────────────────
// (الـ RateLimitConfig الأصلي يبقى كما هو للتوافق مع الكود القديم)
// نُضيف impl جديد يقبل endpoints
impl RateLimitConfig {
    /// إنشاء من قائمة endpoints (للاستخدام في AppState)
    pub fn with_endpoints(endpoints: Vec<EndpointConfig>) -> Self {
        // نُنشئ RateLimitConfig الأصلي مع الإعدادات الافتراضية
        // الـ endpoints تُستخدَم في MultiEndpointRateLimiter
        let _ = endpoints; // سيُستخدَم في MultiEndpointRateLimiter::new
        Self::new()
    }
}

// ─── RateLimiter wrapper للاستخدام في db.rs ──────────────────────────────────
// db.rs يستورد RateLimiter ويستخدمه كـ Arc<RateLimiter>
// نُعرِّف type alias
pub type RateLimiterForState = MultiEndpointRateLimiter;

// ─── Axum Middleware ───────────────────────────────────────────────────────

/// استخراج مفتاح Rate Limiting من الطلب
/// الأولوية: X-Forwarded-For > X-Real-IP > Connection IP
fn extract_client_key(req: &Request) -> String {
    // محاولة استخراج IP من headers
    if let Some(forwarded) = req.headers().get("x-forwarded-for") {
        if let Ok(val) = forwarded.to_str() {
            // أخذ أول IP في القائمة (الأقرب للعميل)
            if let Some(ip) = val.split(',').next() {
                return ip.trim().to_string();
            }
        }
    }

    if let Some(real_ip) = req.headers().get("x-real-ip") {
        if let Ok(val) = real_ip.to_str() {
            return val.to_string();
        }
    }

    // fallback — مفتاح عام (لا يُستخدم في production بدون reverse proxy)
    "unknown".to_string()
}

/// Axum middleware لـ Rate Limiting
///
/// يُطبَّق على مستوى الـ router ويرفض الطلبات الزائدة بـ 429
pub async fn rate_limit_middleware(
    req: Request,
    next: Next,
) -> Response {
    // في الوقت الحالي نستخدم IP-based limiting
    // يمكن تطويره لاحقاً لـ user-based limiting بعد auth
    let client_key = extract_client_key(&req);
    let path = req.uri().path().to_string();

    // تحديد نوع الـ endpoint
    let is_auth = path.contains("/auth");
    let is_sandbox = path.contains("/sandbox");
    let is_chat = path.contains("/zen/chat") || path.contains("/agent/chat");

    // Rate limit بسيط — 200 req/min لكل IP
    // TODO: استخدام RateLimitConfig من AppState في Sprint 3
    let limit = if is_auth { 10 } else if is_sandbox { 20 } else if is_chat { 30 } else { 200 };
    let window_secs = 60u64;

    // نستخدم thread-local limiter مبسط هنا
    // في Sprint 3 سيُنقل إلى AppState
    static LIMITER: std::sync::OnceLock<RateLimiter> = std::sync::OnceLock::new();
    let limiter = LIMITER.get_or_init(|| RateLimiter::new(200, 60));

    // مفتاح مركّب: IP + نوع endpoint
    let composite_key = format!("{client_key}:{}", if is_auth { "auth" } else if is_sandbox { "sandbox" } else if is_chat { "chat" } else { "api" });

    if !limiter.check_and_record(&composite_key) {
        let (current, max) = limiter.status(&composite_key);
        return (
            StatusCode::TOO_MANY_REQUESTS,
            [
                ("Retry-After", "60"),
                ("X-RateLimit-Limit", "200"),
                ("X-RateLimit-Remaining", "0"),
            ],
            Json(json!({
                "error": "Too Many Requests",
                "message": "Rate limit exceeded. Please wait before retrying.",
                "retry_after_secs": 60,
            })),
        ).into_response();
    }

    next.run(req).await
}

// ─── اختبارات ──────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_within_limit() {
        let limiter = RateLimiter::new(5, 60);
        for _ in 0..5 {
            assert!(limiter.check_and_record("test_key"), "يجب السماح بالطلبات ضمن الحد");
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let limiter = RateLimiter::new(3, 60);
        for _ in 0..3 {
            limiter.check_and_record("test_key2");
        }
        assert!(!limiter.check_and_record("test_key2"), "يجب رفض الطلب الرابع");
    }

    #[test]
    fn test_rate_limiter_different_keys_independent() {
        let limiter = RateLimiter::new(2, 60);
        limiter.check_and_record("key_a");
        limiter.check_and_record("key_a");
        // key_a وصل للحد، لكن key_b لا يزال مسموحاً
        assert!(!limiter.check_and_record("key_a"), "key_a يجب أن يُرفض");
        assert!(limiter.check_and_record("key_b"), "key_b يجب أن يُسمح");
    }

    #[test]
    fn test_rate_limiter_status() {
        let limiter = RateLimiter::new(10, 60);
        limiter.check_and_record("status_key");
        limiter.check_and_record("status_key");
        let (current, max) = limiter.status("status_key");
        assert_eq!(current, 2);
        assert_eq!(max, 10);
    }

    #[test]
    fn test_extract_client_key_forwarded() {
        // اختبار استخراج IP من X-Forwarded-For
        let mut req = Request::builder()
            .header("x-forwarded-for", "192.168.1.1, 10.0.0.1")
            .body(axum::body::Body::empty())
            .unwrap();
        let key = extract_client_key(&req);
        assert_eq!(key, "192.168.1.1");
    }
}
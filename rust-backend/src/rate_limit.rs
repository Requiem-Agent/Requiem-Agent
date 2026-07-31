// rate_limit.rs — Sliding-window rate limiter with per-user AND per-IP support
// S4-02: upgraded from per-IP only to per-user (JWT user_id) + per-IP fallback
//
// Architecture:
//   RateLimitKey = UserKey(user_id) | IpKey(ip_addr)
//   MultiEndpointRateLimiter holds one SlidingWindowLimiter per endpoint
//   Axum middleware extracts JWT user_id first; falls back to IP if unauthenticated

use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};
use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tracing::{debug, warn};

use crate::error::AppError;

// ─────────────────────────────────────────────
// Rate-limit key: user ID (authenticated) or IP (anonymous)
// ─────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RateLimitKey {
    /// Authenticated user — identified by JWT `sub` claim
    User(String),
    /// Anonymous request — identified by remote IP
    Ip(IpAddr),
    /// Fallback when neither is available (e.g. unix socket)
    Unknown(String),
}

impl RateLimitKey {
    pub fn as_str(&self) -> String {
        match self {
            RateLimitKey::User(id) => format!("user:{}", id),
            RateLimitKey::Ip(ip) => format!("ip:{}", ip),
            RateLimitKey::Unknown(s) => format!("unknown:{}", s),
        }
    }
}

// ─────────────────────────────────────────────
// Per-key sliding-window state
// ─────────────────────────────────────────────

#[derive(Debug)]
struct WindowState {
    timestamps: Vec<Instant>,
    last_seen: Instant,
}

impl WindowState {
    fn new() -> Self {
        Self {
            timestamps: Vec::new(),
            last_seen: Instant::now(),
        }
    }

    fn check_and_record(&mut self, max_requests: u32, window: Duration) -> bool {
        let now = Instant::now();
        self.last_seen = now;
        let cutoff = now - window;
        self.timestamps.retain(|&t| t > cutoff);
        if self.timestamps.len() < max_requests as usize {
            self.timestamps.push(now);
            true
        } else {
            false
        }
    }
}

// ─────────────────────────────────────────────
// Single-endpoint sliding-window limiter
// ─────────────────────────────────────────────

#[derive(Debug)]
pub struct SlidingWindowLimiter {
    max_requests: u32,
    window: Duration,
    state: Mutex<HashMap<RateLimitKey, WindowState>>,
}

impl SlidingWindowLimiter {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window: Duration::from_secs(window_secs),
            state: Mutex::new(HashMap::new()),
        }
    }

    pub fn check(&self, key: &RateLimitKey) -> Result<(), AppError> {
        let mut map = self.state.lock().map_err(|_| {
            AppError::Internal("rate limiter mutex poisoned".into())
        })?;
        let entry = map.entry(key.clone()).or_insert_with(WindowState::new);
        if entry.check_and_record(self.max_requests, self.window) {
            debug!(key = %key.as_str(), "rate limit OK");
            Ok(())
        } else {
            warn!(
                key = %key.as_str(),
                max = self.max_requests,
                window_secs = self.window.as_secs(),
                "rate limit exceeded"
            );
            Err(AppError::RateLimit(format!(
                "Rate limit exceeded: max {} requests per {} seconds",
                self.max_requests,
                self.window.as_secs()
            )))
        }
    }

    pub fn gc(&self) {
        if let Ok(mut map) = self.state.lock() {
            let cutoff = Instant::now() - self.window * 2;
            map.retain(|_, v| v.last_seen > cutoff);
        }
    }

    pub fn tracked_keys(&self) -> usize {
        self.state.lock().map(|m| m.len()).unwrap_or(0)
    }
}

// ─────────────────────────────────────────────
// Multi-endpoint limiter
// ─────────────────────────────────────────────

#[derive(Debug)]
pub struct MultiEndpointRateLimiter {
    pub auth: SlidingWindowLimiter,
    pub sandbox: SlidingWindowLimiter,
    pub chat: SlidingWindowLimiter,
    pub api: SlidingWindowLimiter,
}

impl MultiEndpointRateLimiter {
    pub fn from_env() -> Self {
        let auth_max = std::env::var("RATE_AUTH_MAX")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(10u32);
        let sandbox_max = std::env::var("RATE_SANDBOX_MAX")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(20u32);
        let chat_max = std::env::var("RATE_CHAT_MAX")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(30u32);
        let api_max = std::env::var("RATE_API_MAX")
            .ok().and_then(|v| v.parse().ok()).unwrap_or(200u32);

        tracing::info!(auth_max, sandbox_max, chat_max, api_max,
            "MultiEndpointRateLimiter initialised");

        Self {
            auth: SlidingWindowLimiter::new(auth_max, 60),
            sandbox: SlidingWindowLimiter::new(sandbox_max, 60),
            chat: SlidingWindowLimiter::new(chat_max, 60),
            api: SlidingWindowLimiter::new(api_max, 60),
        }
    }

    pub fn classify_path(path: &str) -> &'static str {
        if path.starts_with("/auth") || path.starts_with("/login") || path.starts_with("/register") {
            "auth"
        } else if path.starts_with("/sandbox") || path.starts_with("/execute") {
            "sandbox"
        } else if path.starts_with("/chat") || path.starts_with("/agent") || path.starts_with("/ws") {
            "chat"
        } else {
            "api"
        }
    }

    pub fn check(&self, path: &str, key: &RateLimitKey) -> Result<(), AppError> {
        match Self::classify_path(path) {
            "auth"    => self.auth.check(key),
            "sandbox" => self.sandbox.check(key),
            "chat"    => self.chat.check(key),
            _         => self.api.check(key),
        }
    }

    pub fn gc_all(&self) {
        self.auth.gc();
        self.sandbox.gc();
        self.chat.gc();
        self.api.gc();
    }
}

// ─────────────────────────────────────────────
// Trait for AppState integration
// ─────────────────────────────────────────────

pub trait HasRateLimiter {
    fn rate_limiter(&self) -> &Arc<MultiEndpointRateLimiter>;
}

// ─────────────────────────────────────────────
// Key extraction from Axum request
// ─────────────────────────────────────────────

/// Extract rate-limit key:
///   1. `X-User-Id` header (set by JWT middleware after token validation)
///   2. `X-Forwarded-For` / `X-Real-IP` (behind reverse proxy)
///   3. Socket peer address (ConnectInfo extension)
pub fn extract_rate_limit_key(req: &Request<Body>) -> RateLimitKey {
    // 1. Authenticated user ID injected by JWT middleware
    if let Some(user_id) = req
        .headers()
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .filter(|s| !s.is_empty())
    {
        return RateLimitKey::User(user_id.to_string());
    }

    // 2. Forwarded IP
    if let Some(forwarded) = req.headers().get("x-forwarded-for").and_then(|v| v.to_str().ok()) {
        if let Some(ip_str) = forwarded.split(',').next().map(str::trim) {
            if let Ok(ip) = ip_str.parse::<IpAddr>() {
                return RateLimitKey::Ip(ip);
            }
        }
    }
    if let Some(real_ip) = req.headers().get("x-real-ip").and_then(|v| v.to_str().ok()) {
        if let Ok(ip) = real_ip.parse::<IpAddr>() {
            return RateLimitKey::Ip(ip);
        }
    }

    // 3. Socket address
    if let Some(addr) = req
        .extensions()
        .get::<axum::extract::ConnectInfo<std::net::SocketAddr>>()
    {
        return RateLimitKey::Ip(addr.0.ip());
    }

    RateLimitKey::Unknown("no-key".into())
}

// ─────────────────────────────────────────────
// Axum middleware
// ─────────────────────────────────────────────

pub async fn rate_limit_middleware<S>(
    State(state): State<Arc<S>>,
    req: Request<Body>,
    next: Next,
) -> Response
where
    S: HasRateLimiter + Send + Sync + 'static,
{
    let path = req.uri().path().to_string();
    let key = extract_rate_limit_key(&req);

    match state.rate_limiter().check(&path, &key) {
        Ok(()) => next.run(req).await,
        Err(AppError::RateLimit(msg)) => {
            crate::metrics::record_rate_limit_hit(
                MultiEndpointRateLimiter::classify_path(&path),
            );
            (
                StatusCode::TOO_MANY_REQUESTS,
                [
                    ("Retry-After", "60"),
                    ("X-RateLimit-Key", &key.as_str()),
                    ("Content-Type", "application/json"),
                ],
                format!(
                    r#"{{"error":"rate_limit_exceeded","message":"{}","retry_after":60}}"#,
                    msg
                ),
            )
                .into_response()
        }
        Err(e) => e.into_response(),
    }
}

// ─────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn user_key(id: &str) -> RateLimitKey { RateLimitKey::User(id.to_string()) }
    fn ip_key(ip: &str) -> RateLimitKey { RateLimitKey::Ip(ip.parse().unwrap()) }

    #[test]
    fn test_allows_within_limit() {
        let lim = SlidingWindowLimiter::new(5, 60);
        let key = user_key("u1");
        for _ in 0..5 { assert!(lim.check(&key).is_ok()); }
    }

    #[test]
    fn test_blocks_over_limit() {
        let lim = SlidingWindowLimiter::new(3, 60);
        let key = user_key("u2");
        for _ in 0..3 { lim.check(&key).unwrap(); }
        assert!(lim.check(&key).is_err());
    }

    #[test]
    fn test_per_user_isolation() {
        let lim = SlidingWindowLimiter::new(2, 60);
        let alice = user_key("alice");
        let bob   = user_key("bob");
        lim.check(&alice).unwrap();
        lim.check(&alice).unwrap();
        assert!(lim.check(&alice).is_err(), "alice blocked");
        assert!(lim.check(&bob).is_ok(),   "bob unaffected");
    }

    #[test]
    fn test_per_ip_isolation() {
        let lim = SlidingWindowLimiter::new(2, 60);
        let a = ip_key("192.168.1.1");
        let b = ip_key("10.0.0.1");
        lim.check(&a).unwrap();
        lim.check(&a).unwrap();
        assert!(lim.check(&a).is_err());
        assert!(lim.check(&b).is_ok());
    }

    #[test]
    fn test_path_classification() {
        assert_eq!(MultiEndpointRateLimiter::classify_path("/auth/login"),   "auth");
        assert_eq!(MultiEndpointRateLimiter::classify_path("/sandbox/run"),  "sandbox");
        assert_eq!(MultiEndpointRateLimiter::classify_path("/chat/send"),    "chat");
        assert_eq!(MultiEndpointRateLimiter::classify_path("/agent/loop"),   "chat");
        assert_eq!(MultiEndpointRateLimiter::classify_path("/ws/stream"),    "chat");
        assert_eq!(MultiEndpointRateLimiter::classify_path("/bots/list"),    "api");
        assert_eq!(MultiEndpointRateLimiter::classify_path("/metrics"),      "api");
    }

    #[test]
    fn test_key_display() {
        assert_eq!(user_key("abc").as_str(), "user:abc");
        assert_eq!(ip_key("127.0.0.1").as_str(), "ip:127.0.0.1");
        assert_eq!(RateLimitKey::Unknown("x".into()).as_str(), "unknown:x");
    }

    #[test]
    fn test_gc_removes_stale_entries() {
        let lim = SlidingWindowLimiter::new(100, 1);
        let key = user_key("gc-test");
        lim.check(&key).unwrap();
        assert_eq!(lim.tracked_keys(), 1);
        std::thread::sleep(Duration::from_millis(2100));
        lim.gc();
        assert_eq!(lim.tracked_keys(), 0, "GC should remove stale entry");
    }
}

// db_pool.rs — S9-05: Database Connection Pooling Optimization + Query Performance Monitoring
// يُحسّن إعدادات الـ connection pool ويُراقب أداء الـ queries

use std::time::{Duration, Instant};
use tracing::{debug, info, warn};

// ─────────────────────────────────────────────────────────────────────────────
// Pool configuration
// ─────────────────────────────────────────────────────────────────────────────

/// إعدادات الـ connection pool المُحسَّنة
#[derive(Debug, Clone)]
pub struct PoolConfig {
    /// أقصى عدد connections متزامنة
    pub max_connections: u32,
    /// أدنى عدد connections (idle pool)
    pub min_connections: u32,
    /// مهلة الانتظار للحصول على connection
    pub acquire_timeout: Duration,
    /// مهلة الـ idle connection قبل إغلاقه
    pub idle_timeout: Duration,
    /// أقصى عمر للـ connection
    pub max_lifetime: Duration,
    /// فترة فحص صحة الـ connections
    pub health_check_interval: Duration,
}

impl Default for PoolConfig {
    fn default() -> Self {
        // قيم مُحسَّنة لـ production
        Self {
            max_connections: parse_env_u32("DB_MAX_CONNECTIONS", 20),
            min_connections: parse_env_u32("DB_MIN_CONNECTIONS", 2),
            acquire_timeout: Duration::from_secs(parse_env_u64("DB_ACQUIRE_TIMEOUT_SECS", 5)),
            idle_timeout: Duration::from_secs(parse_env_u64("DB_IDLE_TIMEOUT_SECS", 600)),
            max_lifetime: Duration::from_secs(parse_env_u64("DB_MAX_LIFETIME_SECS", 1800)),
            health_check_interval: Duration::from_secs(30),
        }
    }
}

fn parse_env_u32(key: &str, default: u32) -> u32 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

fn parse_env_u64(key: &str, default: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

// ─────────────────────────────────────────────────────────────────────────────
// Query performance monitoring
// ─────────────────────────────────────────────────────────────────────────────

/// إحصائيات query واحدة
#[derive(Debug, Clone)]
pub struct QueryStats {
    pub query_name: String,
    pub duration_ms: u64,
    pub rows_affected: u64,
    pub success: bool,
    pub error: Option<String>,
}

/// مراقب أداء الـ queries
pub struct QueryMonitor {
    /// الحد الذي فوقه نُسجّل تحذيراً (بالـ ms)
    pub slow_query_threshold_ms: u64,
}

impl Default for QueryMonitor {
    fn default() -> Self {
        Self {
            slow_query_threshold_ms: parse_env_u64("SLOW_QUERY_THRESHOLD_MS", 100),
        }
    }
}

impl QueryMonitor {
    /// يُنفّذ query ويُسجّل أداءه
    pub async fn track<F, T, E>(&self, query_name: &str, f: F) -> Result<T, E>
    where
        F: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let start = Instant::now();
        let result = f.await;
        let duration_ms = start.elapsed().as_millis() as u64;

        match &result {
            Ok(_) => {
                if duration_ms > self.slow_query_threshold_ms {
                    warn!(
                        query = %query_name,
                        duration_ms = duration_ms,
                        threshold_ms = self.slow_query_threshold_ms,
                        "Slow query detected"
                    );
                } else {
                    debug!(
                        query = %query_name,
                        duration_ms = duration_ms,
                        "Query completed"
                    );
                }
            }
            Err(e) => {
                warn!(
                    query = %query_name,
                    duration_ms = duration_ms,
                    error = %e,
                    "Query failed"
                );
            }
        }

        // تحديث Prometheus metrics
        self.update_metrics(query_name, duration_ms, result.is_ok());

        result
    }

    fn update_metrics(&self, query_name: &str, duration_ms: u64, success: bool) {
        // في الـ production:
        // DB_QUERY_DURATION.with_label_values(&[query_name]).observe(duration_ms as f64 / 1000.0);
        // DB_QUERY_TOTAL.with_label_values(&[query_name, if success { "ok" } else { "error" }]).inc();
        debug!(
            query = %query_name,
            duration_ms = duration_ms,
            success = success,
            "DB metrics updated"
        );
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Pool health checker
// ─────────────────────────────────────────────────────────────────────────────

/// إحصائيات صحة الـ pool
#[derive(Debug, Clone, serde::Serialize)]
pub struct PoolHealth {
    pub total_connections: u32,
    pub idle_connections: u32,
    pub active_connections: u32,
    pub max_connections: u32,
    pub utilization_pct: f32,
    pub is_healthy: bool,
}

impl PoolHealth {
    pub fn new(total: u32, idle: u32, max: u32) -> Self {
        let active = total.saturating_sub(idle);
        let utilization_pct = if max > 0 {
            (active as f32 / max as f32) * 100.0
        } else {
            0.0
        };

        Self {
            total_connections: total,
            idle_connections: idle,
            active_connections: active,
            max_connections: max,
            utilization_pct,
            is_healthy: total > 0 && utilization_pct < 95.0,
        }
    }

    pub fn log_status(&self) {
        if self.utilization_pct > 80.0 {
            warn!(
                active = self.active_connections,
                max = self.max_connections,
                utilization_pct = self.utilization_pct,
                "High DB pool utilization"
            );
        } else {
            info!(
                active = self.active_connections,
                idle = self.idle_connections,
                max = self.max_connections,
                utilization_pct = self.utilization_pct,
                "DB pool status"
            );
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Query builder helpers (لتجنب SQL injection)
// ─────────────────────────────────────────────────────────────────────────────

/// يبني ORDER BY clause آمن (whitelist-based)
pub fn safe_order_by(field: &str, direction: &str) -> Option<String> {
    let allowed_fields = [
        "created_at", "updated_at", "last_message_at",
        "message_count", "total_tokens", "title",
    ];
    let allowed_directions = ["ASC", "DESC"];

    let field = field.to_lowercase();
    let direction = direction.to_uppercase();

    if allowed_fields.contains(&field.as_str()) && allowed_directions.contains(&direction.as_str()) {
        Some(format!("{} {}", field, direction))
    } else {
        warn!("Rejected unsafe ORDER BY: {} {}", field, direction);
        None
    }
}

/// يبني LIMIT/OFFSET آمن
pub fn safe_pagination(page: i64, per_page: i64) -> (i64, i64) {
    let per_page = per_page.clamp(1, 100);
    let page = page.max(1);
    let offset = (page - 1) * per_page;
    (per_page, offset)
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pool_config_defaults() {
        let config = PoolConfig::default();
        assert!(config.max_connections >= 1);
        assert!(config.min_connections <= config.max_connections);
        assert!(config.acquire_timeout.as_secs() > 0);
    }

    #[test]
    fn test_pool_health_healthy() {
        let health = PoolHealth::new(5, 3, 20);
        assert_eq!(health.active_connections, 2);
        assert_eq!(health.idle_connections, 3);
        assert!(health.is_healthy);
        assert!(health.utilization_pct < 50.0);
    }

    #[test]
    fn test_pool_health_high_utilization() {
        let health = PoolHealth::new(19, 0, 20);
        assert_eq!(health.active_connections, 19);
        assert!(health.utilization_pct > 90.0);
        assert!(!health.is_healthy);
    }

    #[test]
    fn test_safe_order_by_valid() {
        let result = safe_order_by("created_at", "desc");
        assert_eq!(result, Some("created_at DESC".to_string()));
    }

    #[test]
    fn test_safe_order_by_invalid_field() {
        let result = safe_order_by("password; DROP TABLE users", "ASC");
        assert!(result.is_none());
    }

    #[test]
    fn test_safe_order_by_invalid_direction() {
        let result = safe_order_by("created_at", "INVALID");
        assert!(result.is_none());
    }

    #[test]
    fn test_safe_pagination() {
        let (limit, offset) = safe_pagination(3, 20);
        assert_eq!(limit, 20);
        assert_eq!(offset, 40);
    }

    #[test]
    fn test_safe_pagination_clamps_per_page() {
        let (limit, _) = safe_pagination(1, 999);
        assert_eq!(limit, 100);
    }

    #[test]
    fn test_safe_pagination_negative_page() {
        let (_, offset) = safe_pagination(-5, 10);
        assert_eq!(offset, 0); // page 1
    }

    #[tokio::test]
    async fn test_query_monitor_tracks_success() {
        let monitor = QueryMonitor { slow_query_threshold_ms: 1000 };
        let result: Result<i32, String> = monitor
            .track("test_query", async { Ok(42) })
            .await;
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_query_monitor_tracks_failure() {
        let monitor = QueryMonitor { slow_query_threshold_ms: 1000 };
        let result: Result<i32, String> = monitor
            .track("failing_query", async { Err("DB error".to_string()) })
            .await;
        assert!(result.is_err());
    }
}

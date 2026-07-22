//! # Database Migration Runner — S3-05
//!
//! يُشغِّل ملفات SQL من مجلد `migrations/` بالترتيب الرقمي.
//! يتتبَّع الـ migrations المُطبَّقة في جدول `schema_migrations`.
//!
//! ## الاستخدام:
//! ```rust
//! migrate::run(&state.conn).await?;
//! ```

use anyhow::{Context, Result};
use libsql::Connection;
use std::sync::Arc;
use tracing::{info, warn};

/// جدول تتبُّع الـ migrations
const MIGRATIONS_TABLE: &str = "schema_migrations";

/// قائمة الـ migrations المُضمَّنة في الـ binary (compile-time)
/// الترتيب مهم — يُطبَّق تصاعدياً
static MIGRATIONS: &[(&str, &str)] = &[
    (
        "001_initial_schema",
        include_str!("../migrations/001_initial_schema.sql"),
    ),
    (
        "002_rag_memory",
        include_str!("../migrations/002_rag_memory.sql"),
    ),
    (
        "003_rate_limits_and_metrics",
        include_str!("../migrations/003_rate_limits_and_metrics.sql"),
    ),
];

// ─── Public API ───────────────────────────────────────────────────────────────

/// تشغيل جميع الـ migrations المعلَّقة بالترتيب
pub async fn run(conn: &Arc<Connection>) -> Result<()> {
    // 1. إنشاء جدول التتبُّع إذا لم يكن موجوداً
    ensure_migrations_table(conn).await?;

    // 2. جلب الـ migrations المُطبَّقة مسبقاً
    let applied = get_applied_migrations(conn).await?;

    // 3. تطبيق الـ migrations الجديدة
    let mut applied_count = 0usize;
    for (name, sql) in MIGRATIONS {
        if applied.contains(&name.to_string()) {
            info!("migration already applied: {name}");
            continue;
        }

        info!("applying migration: {name}");
        apply_migration(conn, name, sql)
            .await
            .with_context(|| format!("migration failed: {name}"))?;

        applied_count += 1;
        info!("migration applied successfully: {name}");
    }

    if applied_count == 0 {
        info!("all migrations already applied — schema is up to date");
    } else {
        info!("applied {applied_count} migration(s) successfully");
    }

    Ok(())
}

/// التحقق من حالة الـ migrations (للـ health endpoint)
pub async fn status(conn: &Arc<Connection>) -> Result<MigrationStatus> {
    ensure_migrations_table(conn).await?;
    let applied = get_applied_migrations(conn).await?;

    let total = MIGRATIONS.len();
    let applied_count = applied.len();
    let pending: Vec<String> = MIGRATIONS
        .iter()
        .filter(|(name, _)| !applied.contains(&name.to_string()))
        .map(|(name, _)| name.to_string())
        .collect();

    let is_up_to_date = pending.is_empty();
    Ok(MigrationStatus {
        total,
        applied: applied_count,
        pending_count: pending.len(),
        pending,
        is_up_to_date,
    })
}

// ─── Status Type ─────────────────────────────────────────────────────────────

#[derive(Debug, serde::Serialize)]
pub struct MigrationStatus {
    pub total: usize,
    pub applied: usize,
    pub pending_count: usize,
    pub pending: Vec<String>,
    pub is_up_to_date: bool,
}

// ─── Internal Helpers ─────────────────────────────────────────────────────────

/// إنشاء جدول تتبُّع الـ migrations
async fn ensure_migrations_table(conn: &Arc<Connection>) -> Result<()> {
    conn.execute_batch(&format!(
        "CREATE TABLE IF NOT EXISTS {MIGRATIONS_TABLE} (
            name        TEXT PRIMARY KEY,
            applied_at  TEXT NOT NULL DEFAULT (datetime('now'))
        );"
    ))
    .await
    .context("failed to create migrations table")?;
    Ok(())
}

/// جلب أسماء الـ migrations المُطبَّقة
async fn get_applied_migrations(conn: &Arc<Connection>) -> Result<Vec<String>> {
    let mut rows = conn
        .query(
            &format!("SELECT name FROM {MIGRATIONS_TABLE} ORDER BY name"),
            (),
        )
        .await
        .context("failed to query applied migrations")?;

    let mut names = Vec::new();
    while let Ok(Some(row)) = rows.next().await {
        if let Ok(name) = row.get::<String>(0) {
            names.push(name);
        }
    }
    Ok(names)
}

/// تطبيق migration واحد وتسجيله
async fn apply_migration(conn: &Arc<Connection>, name: &str, sql: &str) -> Result<()> {
    // تطبيق الـ SQL (قد يحتوي على عدة statements)
    // نُقسِّم على `;` ونُنفِّذ كل statement منفردة لتجنُّب مشاكل libSQL
    for statement in sql.split(';') {
        let stmt = statement.trim();
        if stmt.is_empty() || stmt.starts_with("--") {
            continue;
        }
        conn.execute(stmt, ())
            .await
            .with_context(|| format!("failed to execute statement in {name}: {stmt}"))?;
    }

    // تسجيل الـ migration كمُطبَّق
    conn.execute(
        &format!("INSERT INTO {MIGRATIONS_TABLE} (name) VALUES (?1)"),
        [name],
    )
    .await
    .with_context(|| format!("failed to record migration: {name}"))?;

    Ok(())
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrations_list_not_empty() {
        assert!(!MIGRATIONS.is_empty(), "يجب أن تكون قائمة الـ migrations غير فارغة");
    }

    #[test]
    fn test_migrations_ordered() {
        // التحقق من أن الأسماء مرتَّبة تصاعدياً
        let names: Vec<&str> = MIGRATIONS.iter().map(|(n, _)| *n).collect();
        let mut sorted = names.clone();
        sorted.sort();
        assert_eq!(names, sorted, "الـ migrations يجب أن تكون مرتَّبة أبجدياً/رقمياً");
    }

    #[test]
    fn test_migrations_sql_not_empty() {
        for (name, sql) in MIGRATIONS {
            assert!(!sql.trim().is_empty(), "migration {name} فارغ");
        }
    }

    #[test]
    fn test_migrations_unique_names() {
        let names: Vec<&str> = MIGRATIONS.iter().map(|(n, _)| *n).collect();
        let unique: std::collections::HashSet<&str> = names.iter().copied().collect();
        assert_eq!(names.len(), unique.len(), "أسماء الـ migrations يجب أن تكون فريدة");
    }
}

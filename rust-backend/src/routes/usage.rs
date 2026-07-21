use axum::{extract::State, http::StatusCode, Extension, Json};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::AppState;
use super::UserId;

const QUOTA_READ: i64 = 50_000;
const QUOTA_WRITE: i64 = 20_000;

pub async fn get_usage(
    State(state): State<Arc<AppState>>,
    Extension(UserId(user_id)): Extension<UserId>,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let uid = user_id.clone();

    // ── Quota ──────────────────────────────────────────────────────────────
    let mut rows = state.conn.query(
        "SELECT quota_read_used, quota_write_used, quota_reset_at FROM users WHERE id = ?1",
        [uid.clone()],
    ).await.map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({ "error": e.to_string() }))))?;

    let row = rows.next().await.ok().flatten()
        .ok_or_else(|| (StatusCode::NOT_FOUND, Json(json!({ "error": "User not found" }))))?;

    let read_used: i64 = row.get(0).unwrap_or(0);
    let write_used: i64 = row.get(1).unwrap_or(0);
    let reset_date: String = row.get(2).unwrap_or_default();

    // ── Session count ──────────────────────────────────────────────────────
    let session_count: i64 = {
        let mut r = state.conn.query(
            "SELECT COUNT(*) FROM sessions WHERE user_id = ?1",
            [uid.clone()],
        ).await.unwrap_or_else(|_| unreachable!());
        r.next().await.ok().flatten().and_then(|row| row.get(0).ok()).unwrap_or(0)
    };

    // ── Message count (sum of message_count across sessions) ───────────────
    let message_count: i64 = {
        let mut r = state.conn.query(
            "SELECT COALESCE(SUM(message_count), 0) FROM sessions WHERE user_id = ?1",
            [uid.clone()],
        ).await.unwrap_or_else(|_| unreachable!());
        r.next().await.ok().flatten().and_then(|row| row.get(0).ok()).unwrap_or(0)
    };

    // ── Memory count ───────────────────────────────────────────────────────
    let memory_count: i64 = {
        let mut r = state.conn.query(
            "SELECT COUNT(*) FROM memories WHERE user_id = ?1",
            [uid.clone()],
        ).await.unwrap_or_else(|_| unreachable!());
        r.next().await.ok().flatten().and_then(|row| row.get(0).ok()).unwrap_or(0)
    };

    // ── Storage size ───────────────────────────────────────────────────────
    let storage_bytes = crate::storage::user_storage_usage(&uid).await.unwrap_or(0);

    Ok(Json(json!({
        "quotaRead":          QUOTA_READ,
        "quotaWrite":         QUOTA_WRITE,
        "quotaReadUsed":      read_used,
        "quotaWriteUsed":     write_used,
        "quotaReadRemaining": (QUOTA_READ  - read_used).max(0),
        "quotaWriteRemaining":(QUOTA_WRITE - write_used).max(0),
        "resetDate":          reset_date,
        "sessionCount":       session_count,
        "messageCount":       message_count,
        "memoryCount":        memory_count,
        "storageBytes":       storage_bytes,
    })))
}

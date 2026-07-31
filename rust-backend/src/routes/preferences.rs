// preferences.rs — User preferences CRUD API
// S5-02: GET /api/preferences  → fetch current user's preferences
//        PUT /api/preferences  → update current user's preferences
//
// Table: user_preferences (from migration 004_user_preferences.sql)
// Auth: requires valid JWT — user_id extracted from X-User-Id header (set by auth middleware)

use axum::{
    extract::{Json, State},
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::error::AppError;

// ─────────────────────────────────────────────────────────────────────────────
// Response / Request shapes
// ─────────────────────────────────────────────────────────────────────────────

/// Full user preferences object returned by GET /api/preferences
#[derive(Debug, Serialize, Deserialize)]
pub struct UserPreferences {
    // UI / UX
    #[serde(default = "default_theme")]
    pub theme: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_true")]
    pub compact_mode: bool,
    #[serde(default = "default_true")]
    pub show_timestamps: bool,
    #[serde(default = "default_true")]
    pub enable_animations: bool,

    // Agent behaviour
    #[serde(default = "default_model")]
    pub default_model: String,
    #[serde(default = "default_mode")]
    pub default_mode: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: i32,
    #[serde(default = "default_temperature")]
    pub temperature: f64,
    #[serde(default)]
    pub system_prompt: Option<String>,
    #[serde(default = "default_true")]
    pub stream_responses: bool,
    #[serde(default = "default_true")]
    pub show_thinking: bool,

    // Notifications
    #[serde(default = "default_true")]
    pub notify_on_complete: bool,
    #[serde(default)]
    pub notify_on_error: bool,
    #[serde(default)]
    pub notify_on_mention: bool,

    // Privacy
    #[serde(default = "default_true")]
    pub save_history: bool,
    #[serde(default)]
    pub share_analytics: bool,
}

fn default_theme() -> String { "dark".into() }
fn default_language() -> String { "en".into() }
fn default_true() -> bool { true }
fn default_model() -> String { "claude-sonnet-4-5".into() }
fn default_mode() -> String { "chat".into() }
fn default_max_tokens() -> i32 { 4096 }
fn default_temperature() -> f64 { 0.7 }

/// القيم الافتراضية لـ UserPreferences — تُستخدم عند إنشاء سجل جديد
impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            theme: default_theme(),
            language: default_language(),
            compact_mode: false,
            show_timestamps: true,
            enable_animations: true,
            default_model: default_model(),
            default_mode: default_mode(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            system_prompt: None,
            stream_responses: true,
            show_thinking: false,
            notify_on_complete: true,
            notify_on_error: true,
            notify_on_mention: true,
            save_history: true,
            share_analytics: false,
        }
    }
}

/// Partial update body for PUT /api/preferences
/// All fields are optional — only provided fields are updated (PATCH semantics via PUT)
#[derive(Debug, Deserialize)]
pub struct UpdatePreferencesRequest {
    pub theme: Option<String>,
    pub language: Option<String>,
    pub compact_mode: Option<bool>,
    pub show_timestamps: Option<bool>,
    pub enable_animations: Option<bool>,
    pub default_model: Option<String>,
    pub default_mode: Option<String>,
    pub max_tokens: Option<i32>,
    pub temperature: Option<f64>,
    pub system_prompt: Option<String>,
    pub stream_responses: Option<bool>,
    pub show_thinking: Option<bool>,
    pub notify_on_complete: Option<bool>,
    pub notify_on_error: Option<bool>,
    pub notify_on_mention: Option<bool>,
    pub save_history: Option<bool>,
    pub share_analytics: Option<bool>,
}

impl UpdatePreferencesRequest {
    /// Validate field values (e.g. temperature range, model whitelist)
    pub fn validate(&self) -> Result<(), AppError> {
        if let Some(temp) = self.temperature {
            if !(0.0..=1.0).contains(&temp) {
                return Err(AppError::Validation(
                    "temperature must be between 0.0 and 1.0".into(),
                ));
            }
        }
        if let Some(tokens) = self.max_tokens {
            if tokens < 1 || tokens > 200_000 {
                return Err(AppError::Validation(
                    "max_tokens must be between 1 and 200000".into(),
                ));
            }
        }
        if let Some(ref theme) = self.theme {
            if !["dark", "light", "system"].contains(&theme.as_str()) {
                return Err(AppError::Validation(
                    "theme must be one of: dark, light, system".into(),
                ));
            }
        }
        if let Some(ref mode) = self.default_mode {
            if !["chat", "orchestrator", "code"].contains(&mode.as_str()) {
                return Err(AppError::Validation(
                    "default_mode must be one of: chat, orchestrator, code".into(),
                ));
            }
        }
        Ok(())
    }
}

/// Standard API response wrapper
#[derive(Serialize)]
struct ApiResponse<T: Serialize> {
    success: bool,
    data: T,
}

#[derive(Serialize)]
struct UpdatedResponse {
    message: &'static str,
    updated_fields: Vec<String>,
}

// ─────────────────────────────────────────────────────────────────────────────
// Helper: extract user_id from request headers (set by JWT middleware)
// ─────────────────────────────────────────────────────────────────────────────

fn extract_user_id(headers: &HeaderMap) -> Result<String, AppError> {
    headers
        .get("x-user-id")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::Unauthorized("Missing X-User-Id header".into()))
}

// ─────────────────────────────────────────────────────────────────────────────
// AppState trait — preferences handlers need DB access
// ─────────────────────────────────────────────────────────────────────────────

/// Trait that AppState must implement to support preferences queries.
/// Using a trait keeps this module decoupled from the concrete AppState type.
#[async_trait::async_trait]
pub trait HasPreferencesDb: Send + Sync {
    /// Fetch preferences for a user. Returns defaults if no row exists yet.
    async fn get_preferences(&self, user_id: &str) -> Result<UserPreferences, AppError>;

    /// Upsert preferences for a user. Returns the list of updated field names.
    async fn upsert_preferences(
        &self,
        user_id: &str,
        req: &UpdatePreferencesRequest,
    ) -> Result<Vec<String>, AppError>;
}

// ─────────────────────────────────────────────────────────────────────────────
// GET /api/preferences
// ─────────────────────────────────────────────────────────────────────────────

/// Fetch the current user's preferences.
///
/// Returns defaults if the user has never saved preferences.
///
/// # Response
/// ```json
/// { "success": true, "data": { "theme": "dark", "language": "en", ... } }
/// ```
pub async fn get_preferences<S>(
    State(state): State<Arc<S>>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, AppError>
where
    S: HasPreferencesDb + 'static,
{
    let user_id = extract_user_id(&headers)?;
    info!(user_id = %user_id, "GET /api/preferences");

    let prefs = state.get_preferences(&user_id).await?;

    Ok((
        StatusCode::OK,
        Json(ApiResponse { success: true, data: prefs }),
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// PUT /api/preferences
// ─────────────────────────────────────────────────────────────────────────────

/// Update the current user's preferences (partial update — only provided fields change).
///
/// # Request body
/// ```json
/// { "theme": "light", "temperature": 0.5 }
/// ```
///
/// # Response
/// ```json
/// { "success": true, "data": { "message": "Preferences updated", "updated_fields": ["theme","temperature"] } }
/// ```
pub async fn put_preferences<S>(
    State(state): State<Arc<S>>,
    headers: HeaderMap,
    Json(req): Json<UpdatePreferencesRequest>,
) -> Result<impl IntoResponse, AppError>
where
    S: HasPreferencesDb + 'static,
{
    let user_id = extract_user_id(&headers)?;
    info!(user_id = %user_id, "PUT /api/preferences");

    // Validate before touching the DB
    req.validate()?;

    let updated_fields = state.upsert_preferences(&user_id, &req).await?;

    info!(
        user_id = %user_id,
        fields = ?updated_fields,
        "Preferences updated"
    );

    Ok((
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            data: UpdatedResponse {
                message: "Preferences updated",
                updated_fields,
            },
        }),
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// Concrete DB implementation (SQLite/libSQL via AppState)
// ─────────────────────────────────────────────────────────────────────────────

/// Macro-free helper: build the SET clause and params list from the update request.
/// Returns (set_clause: String, updated_fields: Vec<String>).
pub fn build_update_fields(req: &UpdatePreferencesRequest) -> (String, Vec<String>) {
    let mut clauses: Vec<String> = Vec::new();
    let mut fields: Vec<String> = Vec::new();

    macro_rules! push_field {
        ($field:ident, $col:expr) => {
            if req.$field.is_some() {
                clauses.push(format!("{} = :{}", $col, $col));
                fields.push($col.to_string());
            }
        };
    }

    push_field!(theme, "theme");
    push_field!(language, "language");
    push_field!(compact_mode, "compact_mode");
    push_field!(show_timestamps, "show_timestamps");
    push_field!(enable_animations, "enable_animations");
    push_field!(default_model, "default_model");
    push_field!(default_mode, "default_mode");
    push_field!(max_tokens, "max_tokens");
    push_field!(temperature, "temperature");
    push_field!(system_prompt, "system_prompt");
    push_field!(stream_responses, "stream_responses");
    push_field!(show_thinking, "show_thinking");
    push_field!(notify_on_complete, "notify_on_complete");
    push_field!(notify_on_error, "notify_on_error");
    push_field!(notify_on_mention, "notify_on_mention");
    push_field!(save_history, "save_history");
    push_field!(share_analytics, "share_analytics");

    (clauses.join(", "), fields)
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_temperature_out_of_range() {
        let req = UpdatePreferencesRequest {
            temperature: Some(1.5),
            ..Default::default()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_validate_temperature_valid() {
        let req = UpdatePreferencesRequest {
            temperature: Some(0.7),
            ..Default::default()
        };
        assert!(req.validate().is_ok());
    }

    #[test]
    fn test_validate_max_tokens_out_of_range() {
        let req = UpdatePreferencesRequest {
            max_tokens: Some(0),
            ..Default::default()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_theme() {
        let req = UpdatePreferencesRequest {
            theme: Some("neon".into()),
            ..Default::default()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_validate_valid_theme() {
        for theme in &["dark", "light", "system"] {
            let req = UpdatePreferencesRequest {
                theme: Some(theme.to_string()),
                ..Default::default()
            };
            assert!(req.validate().is_ok(), "theme '{}' should be valid", theme);
        }
    }

    #[test]
    fn test_validate_invalid_mode() {
        let req = UpdatePreferencesRequest {
            default_mode: Some("turbo".into()),
            ..Default::default()
        };
        assert!(req.validate().is_err());
    }

    #[test]
    fn test_build_update_fields_empty() {
        let req = UpdatePreferencesRequest::default();
        let (clause, fields) = build_update_fields(&req);
        assert!(clause.is_empty());
        assert!(fields.is_empty());
    }

    #[test]
    fn test_build_update_fields_partial() {
        let req = UpdatePreferencesRequest {
            theme: Some("light".into()),
            temperature: Some(0.5),
            ..Default::default()
        };
        let (clause, fields) = build_update_fields(&req);
        assert!(clause.contains("theme"));
        assert!(clause.contains("temperature"));
        assert_eq!(fields.len(), 2);
    }

    #[test]
    fn test_user_preferences_defaults() {
        let prefs = UserPreferences::default();
        assert_eq!(prefs.theme, "dark");
        assert_eq!(prefs.language, "en");
        assert!(prefs.stream_responses);
        assert_eq!(prefs.max_tokens, 4096);
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Default impl for UpdatePreferencesRequest (needed for tests)
// ─────────────────────────────────────────────────────────────────────────────

impl Default for UpdatePreferencesRequest {
    fn default() -> Self {
        Self {
            theme: None,
            language: None,
            compact_mode: None,
            show_timestamps: None,
            enable_animations: None,
            default_model: None,
            default_mode: None,
            max_tokens: None,
            temperature: None,
            system_prompt: None,
            stream_responses: None,
            show_thinking: None,
            notify_on_complete: None,
            notify_on_error: None,
            notify_on_mention: None,
            save_history: None,
            share_analytics: None,
        }
    }
}
// ─────────────────────────────────────────────────────────────────────────────
// S7-04: PATCH /api/preferences — explicit partial-update handler
// ─────────────────────────────────────────────────────────────────────────────

/// PATCH /api/preferences — partial update (only provided fields are changed).
///
/// Identical semantics to PUT but registered on the PATCH method so clients
/// can use the correct HTTP verb for partial updates.
///
/// # Request body (all fields optional)
/// ```json
/// { "theme": "light", "temperature": 0.3 }
/// ```
///
/// # Response
/// ```json
/// { "success": true, "data": { "message": "Preferences updated", "updated_fields": ["theme","temperature"] } }
/// ```
pub async fn patch_preferences<S>(
    State(state): State<Arc<S>>,
    headers: HeaderMap,
    Json(req): Json<UpdatePreferencesRequest>,
) -> Result<impl IntoResponse, AppError>
where
    S: HasPreferencesDb + 'static,
{
    let user_id = extract_user_id(&headers)?;
    tracing::info!(user_id = %user_id, "PATCH /api/preferences");

    req.validate()?;

    let updated_fields = state.upsert_preferences(&user_id, &req).await?;

    tracing::info!(
        user_id = %user_id,
        fields = ?updated_fields,
        "Preferences patched"
    );

    Ok((
        StatusCode::OK,
        Json(ApiResponse {
            success: true,
            data: UpdatedResponse {
                message: "Preferences updated",
                updated_fields,
            },
        }),
    ))
}

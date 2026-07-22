// user_api_keys.rs — S6-02: POST/GET/DELETE /api/user-api-keys
//
// يتيح للمستخدمين حفظ مفاتيح LLM providers (Anthropic, OpenAI, Gemini, …)
// مشفّرة بـ AES-256-GCM عبر crypto.rs.
//
// Endpoints:
//   GET    /api/user-api-keys           → قائمة المفاتيح (بدون plaintext)
//   POST   /api/user-api-keys           → حفظ/تحديث مفتاح provider
//   DELETE /api/user-api-keys/{id}      → حذف مفتاح
//   POST   /api/user-api-keys/decrypt   → فك تشفير مفتاح للاستخدام (داخلي)

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tracing::{error, info};

use crate::{
    crypto::{encrypt_api_key, decrypt_api_key},
    error::AppError,
    routes::AuthUser,
    AppState,
};

// ─── Trait ────────────────────────────────────────────────────────────────────

/// Trait يُجرّد عمليات قاعدة البيانات لـ user_api_keys.
/// يُمكّن الاختبار بدون DB حقيقي.
#[allow(async_fn_in_trait)]
pub trait HasApiKeysDb: Send + Sync {
    async fn list_api_keys(&self, user_id: &str) -> Result<Vec<StoredApiKey>, AppError>;
    async fn save_api_key(
        &self,
        user_id: &str,
        provider: &str,
        encrypted_key: &str,
        key_hint: &str,
    ) -> Result<StoredApiKey, AppError>;
    async fn delete_api_key(&self, user_id: &str, key_id: &str) -> Result<(), AppError>;
    async fn get_encrypted_key(
        &self,
        user_id: &str,
        provider: &str,
    ) -> Result<Option<String>, AppError>;
}

// ─── Models ───────────────────────────────────────────────────────────────────

/// سجل مفتاح API كما يُخزَّن في DB (encrypted_key لا يُرسَل للعميل)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredApiKey {
    pub id: String,
    pub provider: String,
    /// آخر 4 أحرف من المفتاح الأصلي — للعرض فقط
    pub key_hint: String,
    /// المفتاح المشفّر بـ AES-256-GCM (base64) — لا يُرسَل للعميل
    #[serde(skip_serializing)]
    pub encrypted_key: String,
    pub created_at: String,
    pub updated_at: String,
}

/// الاستجابة الآمنة للعميل (بدون encrypted_key)
#[derive(Debug, Serialize)]
pub struct ApiKeyResponse {
    pub id: String,
    pub provider: String,
    pub key_hint: String,
    pub created_at: String,
    pub updated_at: String,
}

impl From<StoredApiKey> for ApiKeyResponse {
    fn from(k: StoredApiKey) -> Self {
        Self {
            id: k.id,
            provider: k.provider,
            key_hint: k.key_hint,
            created_at: k.created_at,
            updated_at: k.updated_at,
        }
    }
}

/// طلب حفظ مفتاح جديد
#[derive(Debug, Deserialize)]
pub struct SaveApiKeyRequest {
    /// اسم الـ provider: "anthropic" | "openai" | "gemini" | "mistral" | "groq"
    pub provider: String,
    /// المفتاح الأصلي (plaintext) — يُشفَّر فوراً ولا يُخزَّن
    pub api_key: String,
}

/// طلب فك تشفير مفتاح (للاستخدام الداخلي من handlers أخرى)
#[derive(Debug, Deserialize)]
pub struct DecryptKeyRequest {
    pub provider: String,
}

/// استجابة فك التشفير
#[derive(Debug, Serialize)]
pub struct DecryptKeyResponse {
    pub provider: String,
    /// المفتاح الأصلي — يُرسَل مرة واحدة فقط ولا يُخزَّن في الـ response
    pub api_key: String,
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

/// GET /api/user-api-keys
/// يُعيد قائمة مفاتيح المستخدم (بدون plaintext أو encrypted_key)
pub async fn list_api_keys<S>(
    State(state): State<Arc<S>>,
    auth: axum::Extension<AuthUser>,
) -> Result<impl IntoResponse, AppError>
where
    S: HasApiKeysDb + 'static,
{
    let user_id = &auth.0.user_id;
    let keys = state.list_api_keys(user_id).await?;

    let response: Vec<ApiKeyResponse> = keys.into_iter().map(Into::into).collect();

    Ok(Json(serde_json::json!({
        "success": true,
        "data": response,
        "count": response.len()
    })))
}

/// POST /api/user-api-keys
/// يحفظ مفتاح API مشفّراً (أو يُحدّث موجوداً لنفس الـ provider)
///
/// Request body:
/// ```json
/// { "provider": "anthropic", "api_key": "sk-ant-api03-..." }
/// ```
pub async fn save_api_key<S>(
    State(state): State<Arc<S>>,
    auth: axum::Extension<AuthUser>,
    Json(body): Json<SaveApiKeyRequest>,
) -> Result<impl IntoResponse, AppError>
where
    S: HasApiKeysDb + 'static,
{
    let user_id = &auth.0.user_id;

    // Validate provider
    let valid_providers = ["anthropic", "openai", "gemini", "mistral", "groq", "cohere", "together"];
    if !valid_providers.contains(&body.provider.as_str()) {
        return Err(AppError::Validation(format!(
            "Unknown provider '{}'. Supported: {}",
            body.provider,
            valid_providers.join(", ")
        )));
    }

    // Validate key format (basic sanity check)
    if body.api_key.len() < 10 {
        return Err(AppError::Validation("API key is too short".to_string()));
    }
    if body.api_key.len() > 512 {
        return Err(AppError::Validation("API key is too long (max 512 chars)".to_string()));
    }

    // بناء key_hint — آخر 4 أحرف فقط
    let key_hint = format!(
        "{}...{}",
        &body.api_key[..body.api_key.len().min(4)],
        &body.api_key[body.api_key.len().saturating_sub(4)..]
    );

    // تشفير المفتاح
    let encrypted = encrypt_api_key(&body.api_key).map_err(|e| {
        error!("Failed to encrypt API key for user {}: {}", user_id, e);
        AppError::Internal("Encryption failed".to_string())
    })?;

    // حفظ في DB
    let stored = state
        .save_api_key(user_id, &body.provider, &encrypted, &key_hint)
        .await?;

    info!(
        "API key saved: user={}, provider={}, hint={}",
        user_id, body.provider, key_hint
    );

    let response: ApiKeyResponse = stored.into();
    Ok((
        StatusCode::CREATED,
        Json(serde_json::json!({
            "success": true,
            "data": response,
            "message": format!("API key for '{}' saved successfully", body.provider)
        })),
    ))
}

/// DELETE /api/user-api-keys/{id}
/// يحذف مفتاح API بالـ id
pub async fn delete_api_key<S>(
    State(state): State<Arc<S>>,
    auth: axum::Extension<AuthUser>,
    Path(key_id): Path<String>,
) -> Result<impl IntoResponse, AppError>
where
    S: HasApiKeysDb + 'static,
{
    let user_id = &auth.0.user_id;

    state.delete_api_key(user_id, &key_id).await?;

    info!("API key deleted: user={}, key_id={}", user_id, key_id);

    Ok(Json(serde_json::json!({
        "success": true,
        "message": "API key deleted"
    })))
}

/// POST /api/user-api-keys/decrypt
/// يفك تشفير مفتاح provider للاستخدام الداخلي (مثلاً قبل إرسال طلب LLM)
///
/// ⚠️ هذا الـ endpoint حساس — يجب تقييده بـ rate limiting صارم
/// ويُستخدم فقط من الـ backend نفسه (server-to-server)
pub async fn decrypt_api_key_handler<S>(
    State(state): State<Arc<S>>,
    auth: axum::Extension<AuthUser>,
    Json(body): Json<DecryptKeyRequest>,
) -> Result<impl IntoResponse, AppError>
where
    S: HasApiKeysDb + 'static,
{
    let user_id = &auth.0.user_id;

    let encrypted = state
        .get_encrypted_key(user_id, &body.provider)
        .await?
        .ok_or_else(|| AppError::NotFound(format!(
            "No API key found for provider '{}'",
            body.provider
        )))?;

    let plaintext = decrypt_api_key(&encrypted).map_err(|e| {
        error!("Failed to decrypt API key for user {}: {}", user_id, e);
        AppError::Internal("Decryption failed".to_string())
    })?;

    // ⚠️ لا تُسجَّل المفاتيح في الـ logs أبداً
    info!("API key decrypted: user={}, provider={}", user_id, body.provider);

    Ok(Json(DecryptKeyResponse {
        provider: body.provider,
        api_key: plaintext.to_string(),
    }))
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tokio::sync::Mutex;

    /// Mock DB للاختبار
    struct MockApiKeysDb {
        keys: Mutex<HashMap<String, StoredApiKey>>,
    }

    impl MockApiKeysDb {
        fn new() -> Self {
            Self { keys: Mutex::new(HashMap::new()) }
        }
    }

    impl HasApiKeysDb for MockApiKeysDb {
        async fn list_api_keys(&self, user_id: &str) -> Result<Vec<StoredApiKey>, AppError> {
            let keys = self.keys.lock().await;
            Ok(keys.values()
                .filter(|k| k.id.starts_with(user_id))
                .cloned()
                .collect())
        }

        async fn save_api_key(
            &self,
            user_id: &str,
            provider: &str,
            encrypted_key: &str,
            key_hint: &str,
        ) -> Result<StoredApiKey, AppError> {
            let key = StoredApiKey {
                id: format!("{}-{}", user_id, provider),
                provider: provider.to_string(),
                key_hint: key_hint.to_string(),
                encrypted_key: encrypted_key.to_string(),
                created_at: "2025-01-01T00:00:00Z".to_string(),
                updated_at: "2025-01-01T00:00:00Z".to_string(),
            };
            self.keys.lock().await.insert(key.id.clone(), key.clone());
            Ok(key)
        }

        async fn delete_api_key(&self, _user_id: &str, key_id: &str) -> Result<(), AppError> {
            self.keys.lock().await.remove(key_id);
            Ok(())
        }

        async fn get_encrypted_key(
            &self,
            user_id: &str,
            provider: &str,
        ) -> Result<Option<String>, AppError> {
            let keys = self.keys.lock().await;
            let id = format!("{}-{}", user_id, provider);
            Ok(keys.get(&id).map(|k| k.encrypted_key.clone()))
        }
    }

    #[test]
    fn test_api_key_response_hides_encrypted_key() {
        let stored = StoredApiKey {
            id: "id-1".to_string(),
            provider: "anthropic".to_string(),
            key_hint: "sk-a...0001".to_string(),
            encrypted_key: "super-secret-encrypted".to_string(),
            created_at: "2025-01-01T00:00:00Z".to_string(),
            updated_at: "2025-01-01T00:00:00Z".to_string(),
        };
        let response: ApiKeyResponse = stored.into();
        let json = serde_json::to_string(&response).unwrap();
        // encrypted_key يجب ألا يظهر في الـ JSON
        assert!(!json.contains("super-secret-encrypted"));
        assert!(json.contains("sk-a...0001"));
    }

    #[test]
    fn test_key_hint_format() {
        let api_key = "sk-ant-api03-abcdefghijklmnop";
        let hint = format!(
            "{}...{}",
            &api_key[..api_key.len().min(4)],
            &api_key[api_key.len().saturating_sub(4)..]
        );
        assert_eq!(hint, "sk-a...mnop");
    }

    #[tokio::test]
    async fn test_mock_save_and_list() {
        let db = MockApiKeysDb::new();
        let stored = db
            .save_api_key("user1", "openai", "enc-key-123", "sk-o...0001")
            .await
            .unwrap();
        assert_eq!(stored.provider, "openai");

        let list = db.list_api_keys("user1").await.unwrap();
        assert_eq!(list.len(), 1);
    }

    #[tokio::test]
    async fn test_mock_delete() {
        let db = MockApiKeysDb::new();
        db.save_api_key("user1", "anthropic", "enc-key-456", "sk-a...0002")
            .await
            .unwrap();

        db.delete_api_key("user1", "user1-anthropic").await.unwrap();
        let list = db.list_api_keys("user1").await.unwrap();
        assert!(list.is_empty());
    }

    #[tokio::test]
    async fn test_get_encrypted_key_returns_none_for_missing() {
        let db = MockApiKeysDb::new();
        let result = db.get_encrypted_key("user1", "gemini").await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn test_get_encrypted_key_returns_value_after_save() {
        let db = MockApiKeysDb::new();
        db.save_api_key("user1", "gemini", "enc-gemini-key", "AI...xyz")
            .await
            .unwrap();
        let result = db.get_encrypted_key("user1", "gemini").await.unwrap();
        assert_eq!(result, Some("enc-gemini-key".to_string()));
    }
}

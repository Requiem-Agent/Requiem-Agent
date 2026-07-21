//! # Auth Module — المصادقة البرمجية الصارمة
//!
//! - التحقق من Telegram initData باستخدام HMAC-SHA256
//! - إنشاء وتحقّق التوكنات الموقّعة لكل مستخدم
//! - رفض initData منتهي الصلاحية (أكثر من 24 ساعة)
//! - عزل هوية المستخدم — كل طلب يحمل user_id مُتحقّق منه

use hmac::{Hmac, Mac};
use sha2::Sha256;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

type HmacSha256 = Hmac<Sha256>;

/// الحد الأقصى لعمر initData من تلغرام (بالثواني)
const INIT_DATA_MAX_AGE_SECS: u64 = 86400; // 24 ساعة

/// مدة صلاحية التوكن (30 يوماً)
const TOKEN_EXPIRY_DAYS: u64 = 30;

// ─── Telegram User Data ──────────────────────────────────────────────────────

/// معلومات المستخدم المستخرجة من initData
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramUser {
    pub id: i64,
    pub first_name: String,
    pub last_name: Option<String>,
    pub username: Option<String>,
    pub language_code: Option<String>,
    pub is_premium: Option<bool>,
    pub photo_url: Option<String>,
}

impl TelegramUser {
    /// استخراج كائن المستخدم من JSON داخل initData
    pub fn from_init_data(init_data: &BTreeMap<String, String>) -> Result<Self, String> {
        let user_json = init_data.get("user").ok_or("Missing user in initData")?;
        let user: TelegramUser = serde_json::from_str(user_json)
            .map_err(|e| format!("Invalid user JSON: {e}"))?;
        Ok(user)
    }
}

// ─── User Session Context ───────────────────────────────────────────────────

/// معلومات المُستخدم المُوثّقة التي تُرفق مع كل طلب
#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    /// معرف المستخدم الفريد (UUID v4)
    pub user_id: String,
    /// معرف تلغرام الرقمي
    pub telegram_id: i64,
    /// الاسم الأول
    pub first_name: String,
    /// التوكن المُستخدم
    pub token: String,
    /// هل هو مستخدم ضيف (بدون initData)
    pub is_guest: bool,
}

// ─── InitData Validation ────────────────────────────────────────────────────

/// التحقق من صحة initData الواردة من Telegram WebApp
///
/// ## الخطوات:
/// 1. استخراج `hash` من الـ query string
/// 2. ترتيب الباراميترات أبجدياً
/// 3. بناء `data_check_string` بالصيغة المطلوبة
/// 4. حساب HMAC-SHA256 باستخدام مفتاح `WebAppData` + bot_token
/// 5. مقارنة الـ hash
/// 6. التحقق من أن `auth_date` ليس قديماً
pub fn validate_telegram_init_data(
    init_data: &str,
    bot_token: &str,
) -> Result<BTreeMap<String, String>, String> {
    let params: Vec<(String, String)> = url::form_urlencoded::parse(init_data.as_bytes())
        .into_owned()
        .collect();

    let hash = params
        .iter()
        .find(|(k, _)| k == "hash")
        .map(|(_, v)| v.clone())
        .ok_or_else(|| "Missing hash in initData".to_string())?;

    let data_map: BTreeMap<String, String> =
        params.into_iter().filter(|(k, _)| k != "hash").collect();

    // 1. تحقق من auth_date expiration
    if let Some(auth_date_str) = data_map.get("auth_date") {
        let auth_date: u64 = auth_date_str.parse()
            .map_err(|_| "Invalid auth_date format".to_string())?;
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if now > auth_date && now - auth_date > INIT_DATA_MAX_AGE_SECS {
            return Err("initData expired (older than 24 hours)".to_string());
        }
    }

    // 2. بناء data_check_string
    let data_check_string = data_map
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("\n");

    // 3. secret_key = HMAC-SHA256("WebAppData", bot_token)
    let mut mac = HmacSha256::new_from_slice(b"WebAppData")
        .map_err(|e| format!("HMAC init: {e}"))?;
    mac.update(bot_token.as_bytes());
    let secret_key = mac.finalize().into_bytes();

    // 4. expected_hash = HMAC-SHA256(data_check_string, secret_key)
    let mut mac2 = HmacSha256::new_from_slice(&secret_key)
        .map_err(|e| format!("HMAC verify: {e}"))?;
    mac2.update(data_check_string.as_bytes());
    let expected = hex::encode(mac2.finalize().into_bytes());

    // 5. مقارنة
    if expected != hash {
        return Err("Invalid initData hash — data tampered".to_string());
    }

    let mut result = data_map;
    result.insert("hash".to_string(), hash);
    Ok(result)
}

// ─── Token Management ───────────────────────────────────────────────────────

/// إنشاء توكن موقع: `base64url(user_id.timestamp.signature)`
///
/// التوقيع: HMAC-SHA256(payload, session_secret)
pub fn generate_token(user_id: &str, secret: &str) -> String {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let payload = format!("{user_id}.{ts}");
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .expect("HMAC key length valid");
    mac.update(payload.as_bytes());
    let sig = hex::encode(mac.finalize().into_bytes());
    let full = format!("{payload}.{sig}");
    base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, full)
}

/// التحقق من صلاحية التوكن واستخراج user_id
///
/// ## التحققات:
/// - فك base64url بنجاح
/// - تنسيق `user_id.timestamp.signature`
/// - تطابق HMAC-SHA256
/// - عدم انتهاء الصلاحية (30 يوماً)
pub fn verify_token(token: &str, secret: &str) -> Option<String> {
    let decoded = base64::Engine::decode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        token,
    ).ok()?;

    let s = String::from_utf8(decoded).ok()?;
    let parts: Vec<&str> = s.splitn(3, '.').collect();
    if parts.len() != 3 {
        return None;
    }

    let (user_id, ts_str, sig) = (parts[0], parts[1], parts[2]);

    // تحقق من التوقيع
    let payload = format!("{user_id}.{ts_str}");
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).ok()?;
    mac.update(payload.as_bytes());
    let expected = hex::encode(mac.finalize().into_bytes());
    if expected != sig {
        return None;
    }

    // تحقق من انتهاء الصلاحية
    let ts_ms: u128 = ts_str.parse().ok()?;
    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let expiry_ms = (TOKEN_EXPIRY_DAYS as u128) * 24 * 60 * 60 * 1000;
    if now_ms - ts_ms > expiry_ms {
        return None;
    }

    Some(user_id.to_string())
}

/// الحصول على وصف مختصر لحالة التوكن (للتسجيل)
pub fn token_info(token: &str) -> String {
    if let Ok(decoded) = base64::Engine::decode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        token,
    ) {
        if let Ok(s) = String::from_utf8(decoded) {
            let parts: Vec<&str> = s.splitn(3, '.').collect();
            if parts.len() == 3 {
                let user_id = parts[0];
                let ts_str = parts[1];
                if let Ok(ts_ms) = ts_str.parse::<u128>() {
                    let now_ms = SystemTime::now()
                        .duration_since(UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_millis();
                    let age_days = (now_ms - ts_ms) / (24 * 60 * 60 * 1000);
                    return format!("user={user_id}, age={age_days}d");
                }
                return format!("user={user_id}");
            }
        }
    }
    "invalid token".to_string()
}

// ─── اختبارات ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_SECRET: &str = "test-secret-key-for-unit-tests";

    #[test]
    fn test_token_roundtrip() {
        let user_id = "550e8400-e29b-41d4-a716-446655440000";
        let token = generate_token(user_id, TEST_SECRET);
        assert!(!token.is_empty());

        let verified = verify_token(&token, TEST_SECRET);
        assert_eq!(verified.as_deref(), Some(user_id));
    }

    #[test]
    fn test_token_wrong_secret() {
        let token = generate_token("user1", "secret1");
        let verified = verify_token(&token, "secret2");
        assert!(verified.is_none());
    }

    #[test]
    fn test_token_tampered() {
        let token = generate_token("user1", TEST_SECRET);
        // تغيير حرف في التوكن
        let mut bytes: Vec<u8> = token.bytes().collect();
        if bytes.len() > 10 {
            bytes[5] = bytes[5].wrapping_add(1);
        }
        let tampered = String::from_utf8(bytes).unwrap();
        let verified = verify_token(&tampered, TEST_SECRET);
        assert!(verified.is_none());
    }

    #[test]
    fn test_token_expired() {
        // ننشئ توكن بوقت منتهي يدوياً
        let old_ts = 1000u128; // عام 1970
        let payload = format!("user1.{old_ts}");
        let mut mac = HmacSha256::new_from_slice(TEST_SECRET.as_bytes()).unwrap();
        mac.update(payload.as_bytes());
        let sig = hex::encode(mac.finalize().into_bytes());
        let full = format!("{payload}.{sig}");
        let token = base64::Engine::encode(&base64::engine::general_purpose::URL_SAFE_NO_PAD, full);

        let verified = verify_token(&token, TEST_SECRET);
        assert!(verified.is_none(), "Expired token should be rejected");
    }

    #[test]
    fn test_telegram_user_parse() {
        let mut data = BTreeMap::new();
        data.insert("user".to_string(),
            r#"{"id":12345,"first_name":"Test","last_name":"User","username":"tester"}"#.to_string());

        let user = TelegramUser::from_init_data(&data).unwrap();
        assert_eq!(user.id, 12345);
        assert_eq!(user.first_name, "Test");
        assert_eq!(user.username, Some("tester".to_string()));
    }
}

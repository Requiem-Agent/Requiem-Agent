//! # Auth Module — المصادقة عبر Popcorn Gateway
//!
//! - نظام تسجيل دخول باسم مستخدم وكلمة سر
//! - كلمات المرور مشفرة بـ Argon2id (معيار OWASP) مع salt عشوائي
//! - إنشاء وتحقّق التوكنات الموقّعة
//! - جلسات مستمرة 30 يوماً
//! - ربط مع قاعدة بيانات Popcorn (customer_accounts)

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, SaltString},
    Argon2, PasswordHasher, PasswordVerifier,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::{SystemTime, UNIX_EPOCH};

/// مدة صلاحية التوكن (30 يوماً)
const TOKEN_EXPIRY_SECS: u64 = 30 * 24 * 3600;

// ─── User Data ─────────────────────────────────────────────────────────────────

/// معلومات المستخدم
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthUser {
    pub user_id: String,
    pub client_id: String,
    pub username: String,
    pub plan: String,
}

// ─── Login Request ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    pub status: String,
    pub token: String,
    pub user: AuthUser,
}

// ─── Token Generation ─────────────────────────────────────────────────────────

/// إنشاء توكن موقع: `popcorn_{user_id}_{issued}_{hash16}`
///
/// - `issued`: وقت الإصدار بالثواني (epoch) — محمول صراحةً داخل التوكن
/// - `hash16`: أول 16 حرفاً من sha256("{user_id}:{issued}:{secret}")
/// - الصلاحية: TOKEN_EXPIRY_SECS (30 يوماً) من وقت الإصدار
pub fn generate_token(user_id: &str, secret: &str) -> String {
    let issued = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let hash = hex::encode(Sha256::digest(
        format!("{}:{}:{}", user_id, issued, secret).as_bytes(),
    ));
    format!("popcorn_{}_{}_{}", user_id, issued, &hash[..16])
}

/// التحقق من التوكن: تنسيق + توقيع + صلاحية (30 يوماً)
pub fn verify_token(token: &str, secret: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('_').collect();
    if parts.len() != 4 || parts[0] != "popcorn" {
        return None;
    }
    let (user_id, issued_str, hash) = (parts[1], parts[2], parts[3]);
    if hash.len() != 16 {
        return None;
    }
    let issued: u64 = issued_str.parse().ok()?;

    // إعادة حساب التوقيع
    let expected = hex::encode(Sha256::digest(
        format!("{}:{}:{}", user_id, issued, secret).as_bytes(),
    ));
    if hash != &expected[..16] {
        return None;
    }

    // التحقق من انتهاء الصلاحية
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    if now.saturating_sub(issued) > TOKEN_EXPIRY_SECS {
        return None;
    }

    Some(user_id.to_string())
}

/// Hash كلمة المرور — Argon2id مع salt عشوائي لكل مستخدم
/// الصيغة: $argon2id$v=19$m=19456,t=2,p=1$<salt>$<hash>
pub fn hash_password(password: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|h| h.to_string())
        .unwrap_or_else(|_| {
            // Fallback آمن عند فشل Argon2 (لا نرجع النص الواضح أبداً)
            hex::encode(Sha256::digest(
                format!("popcorn-fallback:{}", password).as_bytes(),
            ))
        })
}

/// التحقق من كلمة المرور ضد hash مخزن
pub fn verify_password(password: &str, hash: &str) -> bool {
    // دعم Argon2
    if let Ok(parsed) = PasswordHash::new(hash) {
        return Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok();
    }
    // دعم النسخ القديمة (SHA-256 fallback)
    let legacy = hex::encode(Sha256::digest(
        format!("popcorn:{}:salt", password).as_bytes(),
    ));
    hash == legacy
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_roundtrip() {
        let secret = "test-secret-key";
        let uid = "user-123";
        let token = generate_token(uid, secret);
        let verified = verify_token(&token, secret);
        assert_eq!(verified, Some(uid.to_string()));
    }

    #[test]
    fn test_token_expired() {
        let secret = "test-secret-key";
        let uid = "user-123";
        let old_issued = (SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs())
            - (TOKEN_EXPIRY_SECS + 60);
        let hash = hex::encode(Sha256::digest(
            format!("{}:{}:{}", uid, old_issued, secret).as_bytes(),
        ));
        let token = format!("popcorn_{}_{}_{}", uid, old_issued, &hash[..16]);
        assert_eq!(verify_token(&token, secret), None, "Expired token should be rejected");
    }

    #[test]
    fn test_token_wrong_secret() {
        let token = generate_token("user-1", "secret-a");
        assert_eq!(verify_token(&token, "secret-b"), None);
    }

    #[test]
    fn test_password_hash() {
        let h1 = hash_password("mypassword");
        let h2 = hash_password("mypassword");
        assert_ne!(h1, h2, "salt عشوائي يجب أن يعطي hash مختلفاً");
        assert!(verify_password("mypassword", &h1), "التحقق يجب أن ينجح");
        assert!(!verify_password("wrong", &h1), "كلمة خاطئة يجب أن تفشل");
    }
}

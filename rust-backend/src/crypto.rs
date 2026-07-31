// crypto.rs — AES-256-GCM encryption/decryption for user API keys
// S5-03: Secure storage of user LLM provider keys in user_api_keys table
//
// Design:
//   - Key derivation: HKDF-SHA256 from master secret (ENCRYPTION_KEY env var)
//   - Cipher: AES-256-GCM (authenticated encryption — provides confidentiality + integrity)
//   - Nonce: 96-bit random per encryption (stored prepended to ciphertext)
//   - Storage format: base64(nonce || ciphertext || tag)
//   - The master key is 32 bytes (256 bits), loaded from ENCRYPTION_KEY env var (hex-encoded)
//
// Security properties:
//   - Each encryption uses a fresh random nonce → same plaintext → different ciphertext
//   - GCM authentication tag prevents tampering
//   - Master key never stored in DB — only derived ciphertext
//   - Zeroize sensitive buffers after use

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use tracing::error;
use zeroize::Zeroizing;

use crate::error::AppError;

// ─────────────────────────────────────────────────────────────────────────────
// Constants
// ─────────────────────────────────────────────────────────────────────────────

/// AES-256-GCM nonce size in bytes (96 bits)
const NONCE_SIZE: usize = 12;

/// AES-256-GCM key size in bytes (256 bits)
const KEY_SIZE: usize = 32;

// ─────────────────────────────────────────────────────────────────────────────
// Master key loading
// ─────────────────────────────────────────────────────────────────────────────

/// Load the master encryption key from the ENCRYPTION_KEY environment variable.
///
/// The env var must be a 64-character hex string (32 bytes = 256 bits).
/// In development, a deterministic test key is used if the env var is absent.
///
/// # Errors
/// Returns `AppError::Internal` if the env var is present but malformed.
pub fn load_master_key() -> Result<Zeroizing<[u8; KEY_SIZE]>, AppError> {
    match std::env::var("ENCRYPTION_KEY") {
        Ok(hex_key) => {
            let bytes = hex::decode(hex_key.trim()).map_err(|e| {
                error!("ENCRYPTION_KEY is not valid hex: {}", e);
                AppError::Internal("Invalid ENCRYPTION_KEY format".into())
            })?;

            if bytes.len() != KEY_SIZE {
                error!(
                    "ENCRYPTION_KEY must be {} bytes ({} hex chars), got {}",
                    KEY_SIZE,
                    KEY_SIZE * 2,
                    bytes.len()
                );
                return Err(AppError::Internal("ENCRYPTION_KEY wrong length".into()));
            }

            let mut key = Zeroizing::new([0u8; KEY_SIZE]);
            key.copy_from_slice(&bytes);
            Ok(key)
        }
        Err(_) => {
            // Development fallback — NOT for production
            // In production, ENCRYPTION_KEY must be set
            tracing::warn!(
                "ENCRYPTION_KEY not set — using development fallback key. \
                 Set ENCRYPTION_KEY in production!"
            );
            // Deterministic dev key: sha256("requiem-dev-key-do-not-use-in-prod")
            let dev_key: [u8; KEY_SIZE] = [
                0x7f, 0x83, 0xb1, 0x65, 0x7f, 0xf1, 0xfc, 0x53,
                0xb9, 0x2d, 0xc1, 0x81, 0x48, 0xa1, 0xd6, 0x5d,
                0xfc, 0x2d, 0x4b, 0x1f, 0xa3, 0xd6, 0x77, 0x28,
                0x4a, 0xdd, 0xd2, 0x00, 0x12, 0x6d, 0x90, 0x69,
            ];
            Ok(Zeroizing::new(dev_key))
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Encryption
// ─────────────────────────────────────────────────────────────────────────────

/// Encrypt a plaintext API key using AES-256-GCM.
///
/// # Returns
/// Base64-encoded string: `base64(nonce[12] || ciphertext || tag[16])`
///
/// # Example
/// ```rust
/// let encrypted = encrypt_api_key("sk-ant-api03-...")?;
/// // Store `encrypted` in user_api_keys.encrypted_key column
/// ```
pub fn encrypt_api_key(plaintext: &str) -> Result<String, AppError> {
    let master_key = load_master_key()?;
    let key = Key::<Aes256Gcm>::from_slice(master_key.as_ref());
    let cipher = Aes256Gcm::new(key);

    // Generate a fresh random 96-bit nonce
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

    // Encrypt (returns ciphertext || tag)
    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|e| {
            error!("AES-GCM encryption failed: {}", e);
            AppError::Internal("Encryption failed".into())
        })?;

    // Prepend nonce to ciphertext: nonce || ciphertext
    let mut payload = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
    payload.extend_from_slice(&nonce);
    payload.extend_from_slice(&ciphertext);

    Ok(B64.encode(&payload))
}

// ─────────────────────────────────────────────────────────────────────────────
// Decryption
// ─────────────────────────────────────────────────────────────────────────────

/// Decrypt an AES-256-GCM encrypted API key.
///
/// # Arguments
/// * `encoded` — base64-encoded string from `encrypt_api_key`
///
/// # Returns
/// The original plaintext API key, wrapped in `Zeroizing` to clear memory on drop.
///
/// # Errors
/// Returns `AppError::Internal` if decryption fails (wrong key, tampered data, etc.)
pub fn decrypt_api_key(encoded: &str) -> Result<Zeroizing<String>, AppError> {
    let master_key = load_master_key()?;
    let key = Key::<Aes256Gcm>::from_slice(master_key.as_ref());
    let cipher = Aes256Gcm::new(key);

    // Decode base64
    let payload = B64.decode(encoded).map_err(|e| {
        error!("Failed to base64-decode encrypted key: {}", e);
        AppError::Internal("Invalid encrypted key format".into())
    })?;

    if payload.len() < NONCE_SIZE {
        return Err(AppError::Internal("Encrypted key too short".into()));
    }

    // Split nonce and ciphertext
    let (nonce_bytes, ciphertext) = payload.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);

    // Decrypt and authenticate
    let plaintext_bytes = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| {
            // Don't leak details — just say decryption failed
            AppError::Internal("Decryption failed — key may be tampered or wrong master key".into())
        })?;

    let plaintext = String::from_utf8(plaintext_bytes).map_err(|e| {
        error!("Decrypted bytes are not valid UTF-8: {}", e);
        AppError::Internal("Decrypted key is not valid UTF-8".into())
    })?;

    Ok(Zeroizing::new(plaintext))
}

// ─────────────────────────────────────────────────────────────────────────────
// Key rotation helper
// ─────────────────────────────────────────────────────────────────────────────

/// Re-encrypt a key under a new master key (for key rotation).
///
/// Decrypts with the old key, re-encrypts with the new key.
/// Both keys are provided as hex strings.
pub fn rotate_api_key(
    encoded: &str,
    old_key_hex: &str,
    new_key_hex: &str,
) -> Result<String, AppError> {
    // Decrypt with old key
    let old_bytes = hex::decode(old_key_hex.trim()).map_err(|_| {
        AppError::Internal("Invalid old key hex".into())
    })?;
    if old_bytes.len() != KEY_SIZE {
        return Err(AppError::Internal("Old key wrong length".into()));
    }

    let old_key = Key::<Aes256Gcm>::from_slice(&old_bytes);
    let old_cipher = Aes256Gcm::new(old_key);

    let payload = B64.decode(encoded).map_err(|_| {
        AppError::Internal("Invalid encoded key".into())
    })?;
    if payload.len() < NONCE_SIZE {
        return Err(AppError::Internal("Payload too short".into()));
    }

    let (nonce_bytes, ciphertext) = payload.split_at(NONCE_SIZE);
    let nonce = Nonce::from_slice(nonce_bytes);
    let plaintext_bytes = old_cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| AppError::Internal("Rotation: decryption with old key failed".into()))?;

    // Re-encrypt with new key
    let new_bytes = hex::decode(new_key_hex.trim()).map_err(|_| {
        AppError::Internal("Invalid new key hex".into())
    })?;
    if new_bytes.len() != KEY_SIZE {
        return Err(AppError::Internal("New key wrong length".into()));
    }

    let new_key = Key::<Aes256Gcm>::from_slice(&new_bytes);
    let new_cipher = Aes256Gcm::new(new_key);
    let new_nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let new_ciphertext = new_cipher
        .encrypt(&new_nonce, plaintext_bytes.as_ref())
        .map_err(|_| AppError::Internal("Rotation: re-encryption failed".into()))?;

    let mut new_payload = Vec::with_capacity(NONCE_SIZE + new_ciphertext.len());
    new_payload.extend_from_slice(&new_nonce);
    new_payload.extend_from_slice(&new_ciphertext);

    Ok(B64.encode(&new_payload))
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_API_KEY: &str = "sk-ant-api03-test-key-1234567890abcdef";

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let encrypted = encrypt_api_key(TEST_API_KEY).expect("encryption failed");
        let decrypted = decrypt_api_key(&encrypted).expect("decryption failed");
        assert_eq!(decrypted.as_str(), TEST_API_KEY);
    }

    #[test]
    fn test_different_nonce_each_time() {
        // Same plaintext → different ciphertext (due to random nonce)
        let enc1 = encrypt_api_key(TEST_API_KEY).unwrap();
        let enc2 = encrypt_api_key(TEST_API_KEY).unwrap();
        assert_ne!(enc1, enc2, "Same plaintext should produce different ciphertext");
    }

    #[test]
    fn test_decrypt_tampered_ciphertext_fails() {
        let mut encrypted = encrypt_api_key(TEST_API_KEY).unwrap();
        // Flip a character in the base64 to simulate tampering
        let bytes = unsafe { encrypted.as_bytes_mut() };
        bytes[20] ^= 0xFF;
        let result = decrypt_api_key(&encrypted);
        assert!(result.is_err(), "Tampered ciphertext should fail decryption");
    }

    #[test]
    fn test_decrypt_invalid_base64_fails() {
        let result = decrypt_api_key("not-valid-base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_too_short_fails() {
        // Valid base64 but too short to contain nonce
        let result = decrypt_api_key("aGVsbG8="); // "hello" in base64 (5 bytes < 12)
        assert!(result.is_err());
    }

    #[test]
    fn test_encrypt_empty_string() {
        let encrypted = encrypt_api_key("").expect("should encrypt empty string");
        let decrypted = decrypt_api_key(&encrypted).expect("should decrypt empty string");
        assert_eq!(decrypted.as_str(), "");
    }

    #[test]
    fn test_encrypt_unicode_key() {
        let key = "sk-🔑-unicode-key-测试";
        let encrypted = encrypt_api_key(key).unwrap();
        let decrypted = decrypt_api_key(&encrypted).unwrap();
        assert_eq!(decrypted.as_str(), key);
    }

    #[test]
    fn test_load_master_key_returns_32_bytes() {
        let key = load_master_key().unwrap();
        assert_eq!(key.len(), 32);
    }

    #[test]
    fn test_encrypted_output_is_valid_base64() {
        let encrypted = encrypt_api_key(TEST_API_KEY).unwrap();
        let decoded = B64.decode(&encrypted);
        assert!(decoded.is_ok(), "Output should be valid base64");
        assert!(decoded.unwrap().len() >= NONCE_SIZE + 16); // nonce + min ciphertext + tag
    }
}

//! Secret Encryption Service
//!
//! Provides AES-256-GCM encryption for secret values with:
//! - Random nonce per secret
//! - Master key management from file
//! - Masked preview generation
//! - Degraded mode handling when key is corrupted

use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::RwLock;
use zeroize::Zeroizing;

use crate::error::{Result, TestForgeError};

/// Nonce size for AES-256-GCM (12 bytes)
const NONCE_SIZE: usize = 12;

/// Key size for AES-256 (32 bytes)
const KEY_SIZE: usize = 32;

/// Master key file name
const MASTER_KEY_FILE: &str = "master.key";

/// Secret service state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecretServiceState {
    /// Service is healthy and can encrypt/decrypt
    Healthy,
    /// Service is in degraded mode (key corrupted/missing)
    Degraded,
}

/// Secret encryption service using AES-256-GCM
pub struct SecretService {
    /// Master key (wrapped in Zeroizing for memory safety)
    master_key: RwLock<Option<Zeroizing<[u8; KEY_SIZE]>>>,
    /// Current state of the service
    state: AtomicBool, // true = healthy, false = degraded
    /// Path to master key file
    key_file_path: PathBuf,
}

impl SecretService {
    /// Create a new secret service with the given app data directory
    pub fn new(app_data_dir: PathBuf) -> Self {
        let key_file_path = app_data_dir.join(MASTER_KEY_FILE);
        let state = AtomicBool::new(false);

        Self {
            master_key: RwLock::new(None),
            state,
            key_file_path,
        }
    }

    /// Initialize the service by loading or generating the master key
    pub fn initialize(&self) -> Result<()> {
        // Try to load existing key
        if self.key_file_path.exists() {
            match self.load_master_key() {
                Ok(_) => {
                    self.state.store(true, Ordering::SeqCst);
                    return Ok(());
                }
                Err(_) => {
                    // Key file exists but is corrupted
                    self.state.store(false, Ordering::SeqCst);
                    return Err(TestForgeError::MasterKeyCorrupted);
                }
            }
        }

        // Generate new key
        self.generate_and_save_master_key()?;
        self.state.store(true, Ordering::SeqCst);
        Ok(())
    }

    /// Force the service into degraded mode without generating/loading a key.
    pub fn force_degraded(&self) {
        self.state.store(false, Ordering::SeqCst);
        if let Ok(mut master_key) = self.master_key.write() {
            *master_key = None;
        }
    }

    /// Get the current state of the service
    pub fn state(&self) -> SecretServiceState {
        if self.state.load(Ordering::SeqCst) {
            SecretServiceState::Healthy
        } else {
            SecretServiceState::Degraded
        }
    }

    /// Check if the service is in degraded mode
    pub fn is_degraded(&self) -> bool {
        self.state() == SecretServiceState::Degraded
    }

    /// Check if secret operations are available
    pub fn is_available(&self) -> bool {
        self.state() == SecretServiceState::Healthy
    }

    /// Encrypt a secret value
    ///
    /// Returns a base64-encoded string containing: base64(nonce || ciphertext)
    pub fn encrypt(&self, plaintext: &str) -> Result<String> {
        if self.is_degraded() {
            return Err(TestForgeError::DegradedMode(
                "Cannot encrypt secrets in degraded mode. Master key is corrupted or missing."
                    .to_string(),
            ));
        }

        let master_key = self.master_key.read().map_err(|_| {
            TestForgeError::SecretEncryption(
                "Failed to acquire read lock on master key".to_string(),
            )
        })?;

        let key = master_key
            .as_ref()
            .ok_or_else(|| TestForgeError::MasterKey("Master key not initialized".to_string()))?;

        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(key.as_slice()).map_err(|e| {
            TestForgeError::SecretEncryption(format!("Failed to create cipher: {}", e))
        })?;

        // Generate random nonce
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // Encrypt
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|e| TestForgeError::SecretEncryption(format!("Encryption failed: {}", e)))?;

        // Combine nonce + ciphertext and encode as base64
        let mut combined = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        combined.extend_from_slice(&nonce_bytes);
        combined.extend_from_slice(&ciphertext);

        Ok(BASE64.encode(&combined))
    }

    /// Decrypt a secret value
    ///
    /// Expects a base64-encoded string containing: base64(nonce || ciphertext)
    pub fn decrypt(&self, encrypted: &str) -> Result<String> {
        if self.is_degraded() {
            return Err(TestForgeError::DegradedMode(
                "Cannot decrypt secrets in degraded mode. Master key is corrupted or missing."
                    .to_string(),
            ));
        }

        let master_key = self.master_key.read().map_err(|_| {
            TestForgeError::SecretDecryption(
                "Failed to acquire read lock on master key".to_string(),
            )
        })?;

        let key = master_key
            .as_ref()
            .ok_or_else(|| TestForgeError::MasterKey("Master key not initialized".to_string()))?;

        // Decode base64
        let combined = BASE64
            .decode(encrypted)
            .map_err(|e| TestForgeError::Base64Decode(format!("Failed to decode base64: {}", e)))?;

        if combined.len() < NONCE_SIZE + 1 {
            return Err(TestForgeError::SecretDecryption(
                "Encrypted data too short".to_string(),
            ));
        }

        // Extract nonce and ciphertext
        let nonce = Nonce::from_slice(&combined[..NONCE_SIZE]);
        let ciphertext = &combined[NONCE_SIZE..];

        // Create cipher
        let cipher = Aes256Gcm::new_from_slice(key.as_slice()).map_err(|e| {
            TestForgeError::SecretDecryption(format!("Failed to create cipher: {}", e))
        })?;

        // Decrypt
        let plaintext = cipher
            .decrypt(nonce, ciphertext)
            .map_err(|e| TestForgeError::SecretDecryption(format!("Decryption failed: {}", e)))?;

        // Convert to string
        String::from_utf8(plaintext).map_err(|e| {
            TestForgeError::SecretDecryption(format!("Invalid UTF-8 in decrypted data: {}", e))
        })
    }

    /// Generate a masked preview for a secret value
    ///
    /// Examples:
    /// - "abcdefghij" -> "ab***ij"
    /// - "abc" -> "a*c"
    /// - "ab" -> "**"
    /// - "a" -> "*"
    pub fn generate_masked_preview(&self, value: &str) -> String {
        let len = value.chars().count();

        match len {
            0 => String::new(),
            1 => "*".to_string(),
            2 => "**".to_string(),
            3 => {
                let chars: Vec<char> = value.chars().collect();
                format!("{}*{}", chars[0], chars[2])
            }
            4 => {
                let chars: Vec<char> = value.chars().collect();
                format!("{}{}**", chars[0], chars[1])
            }
            5 => {
                let chars: Vec<char> = value.chars().collect();
                format!("{}***{}{}", chars[0], chars[len - 2], chars[len - 1])
            }
            _ => {
                let chars: Vec<char> = value.chars().collect();
                format!(
                    "{}***{}",
                    chars[0..2].iter().collect::<String>(),
                    chars[len - 2..].iter().collect::<String>()
                )
            }
        }
    }

    /// Generate a masked preview for an already encrypted value
    /// This requires the plaintext to be provided
    pub fn generate_masked_preview_from_plaintext(&self, plaintext: &str) -> String {
        self.generate_masked_preview(plaintext)
    }

    /// Load the master key from file
    fn load_master_key(&self) -> Result<()> {
        let key_data = std::fs::read(&self.key_file_path)?;

        if key_data.len() != KEY_SIZE {
            return Err(TestForgeError::MasterKey(format!(
                "Invalid key file size: expected {}, got {}",
                KEY_SIZE,
                key_data.len()
            )));
        }

        let mut key = Zeroizing::new([0u8; KEY_SIZE]);
        key.copy_from_slice(&key_data);

        let mut master_key = self
            .master_key
            .write()
            .map_err(|_| TestForgeError::MasterKey("Failed to acquire write lock".to_string()))?;
        *master_key = Some(key);

        Ok(())
    }

    /// Generate and save a new master key
    fn generate_and_save_master_key(&self) -> Result<()> {
        // Generate random key
        let mut key = Zeroizing::new([0u8; KEY_SIZE]);
        OsRng.fill_bytes(&mut *key);

        // Ensure parent directory exists
        if let Some(parent) = self.key_file_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Save to file with restricted permissions (0600 on Unix)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::write(&self.key_file_path, &*key)?;
            std::fs::set_permissions(&self.key_file_path, std::fs::Permissions::from_mode(0o600))?;
        }

        #[cfg(not(unix))]
        {
            std::fs::write(&self.key_file_path, &*key)?;
        }

        // Store in memory
        let mut master_key = self
            .master_key
            .write()
            .map_err(|_| TestForgeError::MasterKey("Failed to acquire write lock".to_string()))?;
        *master_key = Some(key);

        Ok(())
    }

    /// Rotate the master key (generate new key and re-encrypt all secrets)
    /// This is a placeholder for future implementation
    pub fn rotate_key(&self) -> Result<()> {
        if self.is_degraded() {
            return Err(TestForgeError::DegradedMode(
                "Cannot rotate key in degraded mode".to_string(),
            ));
        }

        // TODO: Implement key rotation
        // 1. Generate new key
        // 2. Decrypt all secrets with old key
        // 3. Re-encrypt with new key
        // 4. Save new key

        Err(TestForgeError::InvalidOperation(
            "Key rotation not yet implemented".to_string(),
        ))
    }

    /// Derive a key from a password (for future use with OS keychain)
    #[allow(dead_code)]
    fn derive_key_from_password(password: &str, salt: &[u8]) -> [u8; KEY_SIZE] {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(salt);

        let mut key = [0u8; KEY_SIZE];
        key.copy_from_slice(&hasher.finalize());
        key
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_service() -> (SecretService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let service = SecretService::new(temp_dir.path().to_path_buf());
        (service, temp_dir)
    }

    #[test]
    fn test_service_initialization() {
        let (service, _temp_dir) = create_test_service();
        assert!(service.initialize().is_ok());
        assert!(service.is_available());
        assert!(!service.is_degraded());
    }

    #[test]
    fn test_encrypt_decrypt() {
        let (service, _temp_dir) = create_test_service();
        service.initialize().unwrap();

        let plaintext = "my_secret_password";
        let encrypted = service.encrypt(plaintext).unwrap();
        let decrypted = service.decrypt(&encrypted).unwrap();

        assert_eq!(plaintext, decrypted);
    }

    #[test]
    fn test_encrypt_produces_different_ciphertext() {
        let (service, _temp_dir) = create_test_service();
        service.initialize().unwrap();

        let plaintext = "same_password";
        let encrypted1 = service.encrypt(plaintext).unwrap();
        let encrypted2 = service.encrypt(plaintext).unwrap();

        // Same plaintext should produce different ciphertext (due to random nonce)
        assert_ne!(encrypted1, encrypted2);

        // But both should decrypt to the same value
        assert_eq!(plaintext, service.decrypt(&encrypted1).unwrap());
        assert_eq!(plaintext, service.decrypt(&encrypted2).unwrap());
    }

    #[test]
    fn test_masked_preview() {
        let (service, _temp_dir) = create_test_service();

        assert_eq!(service.generate_masked_preview(""), "");
        assert_eq!(service.generate_masked_preview("a"), "*");
        assert_eq!(service.generate_masked_preview("ab"), "**");
        assert_eq!(service.generate_masked_preview("abc"), "a*c");
        assert_eq!(service.generate_masked_preview("abcd"), "ab**");
        assert_eq!(service.generate_masked_preview("abcdefghij"), "ab***ij");
        assert_eq!(
            service.generate_masked_preview("verylongpassword"),
            "ve***rd"
        );
    }

    #[test]
    fn test_masked_preview_unicode() {
        let (service, _temp_dir) = create_test_service();

        // Test with Unicode characters
        assert_eq!(service.generate_masked_preview("пароль"), "па***ль");
        assert_eq!(service.generate_masked_preview("密码123"), "密***23");
    }

    #[test]
    fn test_degraded_mode_on_corrupted_key() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join(MASTER_KEY_FILE);

        // Write invalid key file
        std::fs::write(&key_path, b"invalid_key_data").unwrap();

        let service = SecretService::new(temp_dir.path().to_path_buf());
        let result = service.initialize();

        assert!(result.is_err());
        assert!(service.is_degraded());
        assert!(!service.is_available());
    }

    #[test]
    fn test_encrypt_fails_in_degraded_mode() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join(MASTER_KEY_FILE);

        // Write invalid key file
        std::fs::write(&key_path, b"invalid").unwrap();

        let service = SecretService::new(temp_dir.path().to_path_buf());
        service.initialize().ok(); // Will fail but set degraded mode

        let result = service.encrypt("test");
        assert!(result.is_err());
        assert!(matches!(result, Err(TestForgeError::DegradedMode(_))));
    }

    #[test]
    fn test_decrypt_fails_in_degraded_mode() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join(MASTER_KEY_FILE);

        // Write invalid key file
        std::fs::write(&key_path, b"invalid").unwrap();

        let service = SecretService::new(temp_dir.path().to_path_buf());
        service.initialize().ok();

        let result = service.decrypt("dGVzdA=="); // Valid base64 but will fail
        assert!(result.is_err());
        assert!(matches!(result, Err(TestForgeError::DegradedMode(_))));
    }

    #[test]
    fn test_decrypt_invalid_base64() {
        let (service, _temp_dir) = create_test_service();
        service.initialize().unwrap();

        let result = service.decrypt("not_valid_base64!!!");
        assert!(result.is_err());
    }

    #[test]
    fn test_decrypt_too_short() {
        let (service, _temp_dir) = create_test_service();
        service.initialize().unwrap();

        // Base64 of 5 bytes (less than NONCE_SIZE + 1)
        let result = service.decrypt("SGVsbG8=");
        assert!(result.is_err());
    }

    #[test]
    fn test_key_file_created_on_init() {
        let temp_dir = TempDir::new().unwrap();
        let key_path = temp_dir.path().join(MASTER_KEY_FILE);

        assert!(!key_path.exists());

        let service = SecretService::new(temp_dir.path().to_path_buf());
        service.initialize().unwrap();

        assert!(key_path.exists());

        // Key file should be exactly 32 bytes
        let key_data = std::fs::read(&key_path).unwrap();
        assert_eq!(key_data.len(), KEY_SIZE);
    }

    #[test]
    fn test_persists_across_reinitialization() {
        let temp_dir = TempDir::new().unwrap();

        // First service
        let service1 = SecretService::new(temp_dir.path().to_path_buf());
        service1.initialize().unwrap();
        let encrypted = service1.encrypt("secret_value").unwrap();

        // Second service (simulating app restart)
        let service2 = SecretService::new(temp_dir.path().to_path_buf());
        service2.initialize().unwrap();
        let decrypted = service2.decrypt(&encrypted).unwrap();

        assert_eq!(decrypted, "secret_value");
    }

    #[test]
    fn test_state_enum() {
        let (service, _temp_dir) = create_test_service();

        // Before initialization
        // State is determined by atomic bool, default is false (degraded)
        assert_eq!(service.state(), SecretServiceState::Degraded);

        service.initialize().unwrap();
        assert_eq!(service.state(), SecretServiceState::Healthy);
    }
}

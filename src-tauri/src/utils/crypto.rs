//! Cryptography utilities
//! 
//! Provides helper functions for cryptographic operations.

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};
use rand::RngCore;

/// Generate random bytes of the specified length
pub fn random_bytes(len: usize) -> Vec<u8> {
    let mut bytes = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes
}

/// Encode bytes to base64 string
pub fn encode_base64(data: &[u8]) -> String {
    BASE64.encode(data)
}

/// Decode base64 string to bytes
pub fn decode_base64(encoded: &str) -> Result<Vec<u8>, base64::DecodeError> {
    BASE64.decode(encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_random_bytes_length() {
        let bytes = random_bytes(32);
        assert_eq!(bytes.len(), 32);
        
        let bytes2 = random_bytes(16);
        assert_eq!(bytes2.len(), 16);
    }

    #[test]
    fn test_base64_encode_decode() {
        let original = b"Hello, World!";
        let encoded = encode_base64(original.as_bytes());
        let decoded = decode_base64(&encoded).unwrap();
        assert_eq!(decoded, original.as_bytes());
    }
}

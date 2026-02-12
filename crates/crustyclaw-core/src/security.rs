//! Security invariant assertions and constants.
//!
//! Provides compile-time and const-evaluable security constraints to ensure
//! that key sizes, protocol versions, and other security parameters meet
//! minimum requirements.

/// Minimum acceptable AES key length in bytes (256 bits).
pub const MIN_AES_KEY_BYTES: usize = 32;

/// Minimum acceptable HMAC key length in bytes (256 bits).
pub const MIN_HMAC_KEY_BYTES: usize = 32;

/// Minimum acceptable TLS version (1.2 = 0x0303, 1.3 = 0x0304).
pub const MIN_TLS_VERSION: u16 = 0x0303;

/// Maximum allowed login attempts before lockout.
pub const MAX_LOGIN_ATTEMPTS: u32 = 5;

/// Minimum password length (characters).
pub const MIN_PASSWORD_LENGTH: usize = 12;

/// Assert at compile time that a key size meets the minimum requirement.
///
/// When used as a const initializer (e.g. `const _: () = assert_key_size::<N>();`),
/// the assertion is evaluated at compile time. Undersized keys will cause a
/// build failure.
///
/// # Example
///
/// ```
/// use crustyclaw_core::security;
/// security::assert_key_size::<32>();
/// security::assert_key_size::<64>();
/// ```
pub const fn assert_key_size<const N: usize>() {
    assert!(
        N >= MIN_AES_KEY_BYTES,
        "Key size is below minimum (32 bytes / 256 bits)"
    );
}

/// Assert at compile time that a TLS version meets the minimum requirement.
///
/// # Example
///
/// ```
/// use crustyclaw_core::security;
/// security::assert_tls_version::<0x0303>(); // TLS 1.2
/// security::assert_tls_version::<0x0304>(); // TLS 1.3
/// ```
pub const fn assert_tls_version<const V: u16>() {
    assert!(
        V >= MIN_TLS_VERSION,
        "TLS version is below minimum (TLS 1.2 / 0x0303)"
    );
}

/// A fixed-size buffer for cryptographic keys, enforced at compile time
/// to be at least `MIN_AES_KEY_BYTES`.
///
/// Uses const generics to reject undersized buffers at compile time.
pub struct KeyBuffer<const N: usize> {
    data: [u8; N],
}

impl<const N: usize> Default for KeyBuffer<N> {
    fn default() -> Self {
        Self::new()
    }
}

impl<const N: usize> KeyBuffer<N> {
    /// Create a new zeroed key buffer.
    ///
    /// # Panics
    ///
    /// Compile-time assertion ensures `N >= MIN_AES_KEY_BYTES`.
    pub const fn new() -> Self {
        assert_key_size::<N>();
        Self { data: [0u8; N] }
    }

    /// Create a key buffer from raw bytes.
    pub const fn from_bytes(data: [u8; N]) -> Self {
        assert_key_size::<N>();
        Self { data }
    }

    /// Get a reference to the key data.
    pub fn as_bytes(&self) -> &[u8; N] {
        &self.data
    }

    /// Get a mutable reference to the key data.
    pub fn as_bytes_mut(&mut self) -> &mut [u8; N] {
        &mut self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_buffer_32() {
        let buf = KeyBuffer::<32>::new();
        assert_eq!(buf.as_bytes().len(), 32);
        assert!(buf.as_bytes().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_key_buffer_64() {
        let data = [0xABu8; 64];
        let buf = KeyBuffer::<64>::from_bytes(data);
        assert_eq!(buf.as_bytes()[0], 0xAB);
    }

    #[test]
    fn test_assert_key_size_valid() {
        assert_key_size::<32>();
        assert_key_size::<48>();
        assert_key_size::<64>();
    }

    #[test]
    fn test_assert_tls_version_valid() {
        assert_tls_version::<0x0303>(); // TLS 1.2
        assert_tls_version::<0x0304>(); // TLS 1.3
    }

    #[test]
    fn test_constants() {
        assert_eq!(MIN_AES_KEY_BYTES, 32);
        assert_eq!(MIN_HMAC_KEY_BYTES, 32);
        assert_eq!(MIN_TLS_VERSION, 0x0303);
        assert_eq!(MAX_LOGIN_ATTEMPTS, 5);
        assert_eq!(MIN_PASSWORD_LENGTH, 12);
    }
}

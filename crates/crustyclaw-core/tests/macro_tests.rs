//! Integration tests for crustyclaw-macros derive macros.
//!
//! These live in crustyclaw-core because proc-macro crates can't have
//! integration tests that use their own macros.

#![allow(dead_code)]

use crustyclaw_macros::{Redact, SecureZeroize, Validate};

// ── Redact tests ──────────────────────────────────────────────────

#[derive(Redact)]
struct Credentials {
    pub username: String,
    #[redact]
    pub password: String,
    #[redact]
    pub token: String,
}

#[test]
fn test_redact_debug_output() {
    let creds = Credentials {
        username: "alice".to_string(),
        password: "s3cret".to_string(),
        token: "tok_abc123".to_string(),
    };

    let debug = format!("{creds:?}");
    assert!(debug.contains("alice"), "username should be visible");
    assert!(
        !debug.contains("s3cret"),
        "password should not appear in debug output"
    );
    assert!(
        !debug.contains("tok_abc123"),
        "token should not appear in debug output"
    );
    assert!(
        debug.contains("[REDACTED]"),
        "redacted fields should show [REDACTED]"
    );
}

#[derive(Redact)]
struct NoRedactFields {
    pub name: String,
    pub value: i32,
}

#[test]
fn test_redact_no_redacted_fields() {
    let item = NoRedactFields {
        name: "test".to_string(),
        value: 42,
    };
    let debug = format!("{item:?}");
    assert!(debug.contains("test"));
    assert!(debug.contains("42"));
}

// ── Validate tests ────────────────────────────────────────────────

#[derive(Validate)]
struct ServerConfig {
    #[validate(non_empty)]
    pub host: String,
    #[validate(range(min = 1, max = 65535))]
    pub port: u16,
    #[validate(min_len = 8)]
    pub api_key: String,
    #[validate(max_len = 64)]
    pub description: String,
}

#[test]
fn test_validate_all_valid() {
    let config = ServerConfig {
        host: "localhost".to_string(),
        port: 8080,
        api_key: "abcdefgh".to_string(),
        description: "My server".to_string(),
    };
    assert!(config.validate().is_ok());
}

#[test]
fn test_validate_empty_host() {
    let config = ServerConfig {
        host: "".to_string(),
        port: 8080,
        api_key: "abcdefgh".to_string(),
        description: "desc".to_string(),
    };
    let errors = config.validate().unwrap_err();
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("host"));
    assert!(errors[0].contains("empty"));
}

#[test]
fn test_validate_port_zero() {
    let config = ServerConfig {
        host: "localhost".to_string(),
        port: 0,
        api_key: "abcdefgh".to_string(),
        description: "desc".to_string(),
    };
    let errors = config.validate().unwrap_err();
    assert!(errors.iter().any(|e| e.contains("port")));
}

#[test]
fn test_validate_api_key_too_short() {
    let config = ServerConfig {
        host: "localhost".to_string(),
        port: 8080,
        api_key: "short".to_string(),
        description: "desc".to_string(),
    };
    let errors = config.validate().unwrap_err();
    assert!(errors.iter().any(|e| e.contains("api_key")));
}

#[test]
fn test_validate_description_too_long() {
    let config = ServerConfig {
        host: "localhost".to_string(),
        port: 8080,
        api_key: "abcdefgh".to_string(),
        description: "x".repeat(100),
    };
    let errors = config.validate().unwrap_err();
    assert!(errors.iter().any(|e| e.contains("description")));
}

#[test]
fn test_validate_multiple_errors() {
    let config = ServerConfig {
        host: "".to_string(),
        port: 0,
        api_key: "short".to_string(),
        description: "x".repeat(100),
    };
    let errors = config.validate().unwrap_err();
    assert!(errors.len() >= 3, "should have multiple errors: {errors:?}");
}

// ── SecureZeroize tests ───────────────────────────────────────────

#[derive(SecureZeroize)]
struct Secret {
    pub key: String,
    pub nonce: Vec<u8>,
    #[no_zeroize]
    pub label: String,
}

#[test]
fn test_secure_zeroize_clears_on_drop() {
    // We can't inspect memory after drop, but we can verify the type
    // compiles correctly with the generated Drop impl and doesn't panic.
    let secret = Secret {
        key: "super-secret-key".to_string(),
        nonce: vec![1, 2, 3, 4],
        label: "my-key".to_string(),
    };
    drop(secret); // Should call zeroize on key and nonce, skip label
}

// ── Combined derive test ──────────────────────────────────────────

#[derive(Redact, Validate)]
struct ApiCredential {
    #[validate(non_empty)]
    pub service: String,
    #[redact]
    #[validate(min_len = 16)]
    pub api_key: String,
}

#[test]
fn test_combined_redact_and_validate() {
    let cred = ApiCredential {
        service: "github".to_string(),
        api_key: "ghp_1234567890abcdef".to_string(),
    };

    // Validate passes
    assert!(cred.validate().is_ok());

    // Debug redacts api_key
    let debug = format!("{cred:?}");
    assert!(debug.contains("github"));
    assert!(!debug.contains("ghp_"));
    assert!(debug.contains("[REDACTED]"));
}

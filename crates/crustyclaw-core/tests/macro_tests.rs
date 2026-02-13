//! Integration tests for crustyclaw-macros derive macros.
//!
//! These live in crustyclaw-core because proc-macro crates can't have
//! integration tests that use their own macros.

use crustyclaw_macros::{ActionPlugin, Redact, SecureZeroize, Validate, action_hook};

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

    // Verify fields are populated (exercises field reads)
    assert_eq!(creds.password, "s3cret");
    assert_eq!(creds.token, "tok_abc123");

    // But Debug output should redact them
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
    // Verify the #[no_zeroize] field is accessible and populated
    assert_eq!(secret.label, "my-key");
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

// ── ActionPlugin tests ──────────────────────────────────────────

#[derive(ActionPlugin)]
#[action(name = "greeting", version = "1.0.0", description = "Says hello")]
struct GreetAction {
    #[action_input(required)]
    pub name: String,
    #[action_input(default = "Hello")]
    pub greeting: String,
}

#[test]
fn test_action_plugin_metadata() {
    let action = GreetAction {
        name: "world".to_string(),
        greeting: "Hi".to_string(),
    };
    // Verify fields are accessible
    assert_eq!(action.name, "world");
    assert_eq!(action.greeting, "Hi");

    assert_eq!(GreetAction::plugin_name(), "greeting");
    assert_eq!(GreetAction::plugin_version(), "1.0.0");
    assert_eq!(GreetAction::plugin_description(), "Says hello");
    assert_eq!(GreetAction::input_names(), &["name", "greeting"]);
}

#[derive(ActionPlugin)]
struct MinimalPlugin {
    pub value: String,
}

#[test]
fn test_action_plugin_defaults() {
    let plugin = MinimalPlugin {
        value: "test".to_string(),
    };
    assert_eq!(plugin.value, "test");

    // Without #[action(...)] attrs, uses struct name lowered as name
    assert_eq!(MinimalPlugin::plugin_name(), "minimalplugin");
    assert_eq!(MinimalPlugin::plugin_version(), "0.1.0");
}

// ── action_hook tests ───────────────────────────────────────────

#[action_hook(event = "on_message", priority = 10)]
fn handle_greeting(msg: &str) -> String {
    format!("Hello, {msg}!")
}

#[test]
fn test_action_hook_function_works() {
    assert_eq!(handle_greeting("world"), "Hello, world!");
}

#[test]
fn test_action_hook_registration_const() {
    // The macro generates a const with metadata
    assert_eq!(__HOOK_REG_HANDLE_GREETING.0, "handle_greeting");
    assert_eq!(__HOOK_REG_HANDLE_GREETING.1, "on_message");
    assert_eq!(__HOOK_REG_HANDLE_GREETING.2, 10);
}

// ── security_policy! tests ──────────────────────────────────────

#[test]
fn test_security_policy_macro() {
    let mut engine = crustyclaw_macros::security_policy! {
        allow admin * *;
        allow user read config;
        deny user write secrets [priority = 100];
        deny * * * [priority = 0];
    };

    // Admin can do anything
    assert!(engine.is_allowed("admin", "write", "secrets"));

    // User can read config
    assert!(engine.is_allowed("user", "read", "config"));

    // User denied write to secrets (priority 100 beats allow at 0)
    assert!(!engine.is_allowed("user", "write", "secrets"));

    // Unknown role is denied by default (deny * * *)
    assert!(!engine.is_allowed("guest", "read", "anything"));
}

#[test]
fn test_security_policy_macro_empty() {
    let mut engine = crustyclaw_macros::security_policy! {};

    // No rules = no match = not allowed
    assert!(!engine.is_allowed("admin", "read", "anything"));
}

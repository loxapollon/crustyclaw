#![deny(unsafe_code)]

//! Procedural macros for CrustyClaw.
//!
//! This crate provides derive, attribute, and function-like proc macros:
//!
//! - `#[derive(Redact)]` — auto-redact sensitive fields in Debug output
//! - `#[derive(Validate)]` — generate a `validate()` method from field annotations
//! - `#[derive(SecureZeroize)]` — zeroize sensitive memory on Drop
//! - `#[derive(ActionPlugin)]` — Forgejo Action plugin scaffolding
//! - `#[action_hook(event, priority)]` — hook registration attribute
//! - `security_policy!{}` — DSL for defining security policies

extern crate proc_macro;

mod action_hook;
mod action_plugin;
mod redact;
mod secure_zeroize;
mod security_policy;
mod validate;

use proc_macro::TokenStream;
use syn::{parse_macro_input, DeriveInput};

/// Derive macro for redacting sensitive fields in Debug output.
///
/// Fields annotated with `#[redact]` will display as `[REDACTED]` in the
/// generated `Debug` implementation.
///
/// # Example
///
/// ```ignore
/// use crustyclaw_macros::Redact;
///
/// #[derive(Redact)]
/// struct Credentials {
///     pub username: String,
///     #[redact]
///     pub password: String,
/// }
/// ```
#[proc_macro_derive(Redact, attributes(redact))]
pub fn derive_redact(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    redact::expand(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for compile-time input validation.
///
/// Generates a `validate(&self) -> Result<(), Vec<String>>` method that
/// checks field constraints at runtime, driven by annotations applied at
/// compile time.
///
/// Supported attributes:
/// - `#[validate(non_empty)]` — string/collection must not be empty
/// - `#[validate(range(min = N, max = M))]` — numeric value in [N, M]
/// - `#[validate(min_len = N)]` — minimum length for strings/collections
/// - `#[validate(max_len = N)]` — maximum length for strings/collections
///
/// # Example
///
/// ```ignore
/// use crustyclaw_macros::Validate;
///
/// #[derive(Validate)]
/// struct ServerConfig {
///     #[validate(non_empty)]
///     pub host: String,
///     #[validate(range(min = 1, max = 65535))]
///     pub port: u16,
///     #[validate(min_len = 8)]
///     pub api_key: String,
/// }
/// ```
#[proc_macro_derive(Validate, attributes(validate))]
pub fn derive_validate(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    validate::expand(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for secure memory clearing on drop.
///
/// Generates a `Drop` implementation that calls `zeroize()` on all fields
/// (or only fields *not* marked `#[no_zeroize]`). Requires the `zeroize`
/// crate as a dependency in the consuming crate.
///
/// # Example
///
/// ```ignore
/// use crustyclaw_macros::SecureZeroize;
///
/// #[derive(SecureZeroize)]
/// struct Secret {
///     pub key: String,
///     pub nonce: Vec<u8>,
///     #[no_zeroize]
///     pub label: String,
/// }
/// ```
#[proc_macro_derive(SecureZeroize, attributes(no_zeroize))]
pub fn derive_secure_zeroize(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    secure_zeroize::expand(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Derive macro for Forgejo Action plugin scaffolding.
///
/// Generates `plugin_name()`, `plugin_version()`, `plugin_description()`,
/// `input_names()`, and `from_env()` methods.
///
/// # Example
///
/// ```ignore
/// use crustyclaw_macros::ActionPlugin;
///
/// #[derive(ActionPlugin)]
/// #[action(name = "greeting", version = "1.0.0", description = "Says hello")]
/// struct GreetAction {
///     #[action_input(required)]
///     pub name: String,
///     #[action_input(default = "Hello")]
///     pub greeting: String,
/// }
/// ```
#[proc_macro_derive(ActionPlugin, attributes(action, action_input))]
pub fn derive_action_plugin(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    action_plugin::expand(input)
        .unwrap_or_else(|e| e.to_compile_error())
        .into()
}

/// Attribute macro for registering action hooks.
///
/// Annotates a function as a hook handler for a specific event type,
/// generating static registration metadata.
///
/// # Example
///
/// ```ignore
/// use crustyclaw_macros::action_hook;
///
/// #[action_hook(event = "on_message", priority = 10)]
/// fn handle_greeting(msg: &str) -> String {
///     format!("Hello, {msg}!")
/// }
/// ```
#[proc_macro_attribute]
pub fn action_hook(attr: TokenStream, item: TokenStream) -> TokenStream {
    action_hook::expand(attr, item)
}

/// Function-like proc macro for defining security policies with a DSL.
///
/// Parses a list of `allow`/`deny` rules and generates a `PolicyEngine`.
///
/// # Example
///
/// ```ignore
/// use crustyclaw_macros::security_policy;
///
/// let engine = security_policy! {
///     allow admin * *;
///     allow user read config;
///     deny * write secrets [priority = 100];
/// };
/// ```
#[proc_macro]
pub fn security_policy(input: TokenStream) -> TokenStream {
    security_policy::expand(input)
}

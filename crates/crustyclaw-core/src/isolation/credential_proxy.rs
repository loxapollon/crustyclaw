//! Credential proxying for sandbox isolation.
//!
//! Implements the Docker Sandbox credential proxying pattern: real API
//! keys never enter the sandbox. Instead, sentinel placeholder values
//! are injected into the sandbox environment, and a network-level proxy
//! (or pre-execution hook) swaps sentinels for real credentials on
//! outbound API calls.
//!
//! ## Security model
//!
//! 1. Real credentials stay on the host, managed by [`SecretStore`](crate::secrets::SecretStore).
//! 2. Sandbox receives environment variables with sentinel values (e.g. `__CRUSTYCLAW_SENTINEL_api_key__`).
//! 3. Before forwarding outbound requests, the proxy replaces sentinels with real values.
//! 4. Sandboxed code never sees the real credential — even if it dumps all env vars.
//!
//! ## Usage
//!
//! ```rust,ignore
//! let mut proxy = CredentialProxy::new();
//! proxy.add_mapping("api_key", "sk-real-key-123", "API_KEY");
//!
//! // Inject sentinels into sandbox config
//! let config = proxy.inject_sentinels(sandbox_config);
//!
//! // Later, when intercepting outbound requests:
//! let real_body = proxy.replace_sentinels(&request_body);
//! ```

use std::collections::HashMap;

use super::IsolationError;

/// A mapping from a sentinel placeholder to a real credential.
#[derive(Debug, Clone)]
pub struct SentinelMapping {
    /// Human-readable name of this credential (for logging).
    pub name: String,
    /// The sentinel value injected into the sandbox.
    pub sentinel: String,
    /// The environment variable name inside the sandbox.
    pub env_name: String,
    // Note: the real value is NOT stored here — it stays in SecretStore.
}

/// Manages sentinel-value credential proxying for sandbox environments.
///
/// The proxy holds mappings between sentinel placeholders and credential
/// names. Real credential values are resolved from the
/// [`SecretStore`](crate::secrets::SecretStore) at proxy time, never
/// stored in the proxy itself.
pub struct CredentialProxy {
    /// Sentinel → credential name mappings.
    mappings: Vec<SentinelMapping>,
}

impl CredentialProxy {
    /// Create a new empty credential proxy.
    pub fn new() -> Self {
        Self {
            mappings: Vec::new(),
        }
    }

    /// Generate a deterministic sentinel value for a credential name.
    ///
    /// Sentinel format: `__CRUSTYCLAW_SENTINEL_<name>__`
    fn sentinel_for(name: &str) -> String {
        format!("__CRUSTYCLAW_SENTINEL_{name}__")
    }

    /// Add a credential mapping.
    ///
    /// The `name` is the key used to look up the real value in
    /// [`SecretStore`](crate::secrets::SecretStore). The `env_name`
    /// is the environment variable that will hold the sentinel inside
    /// the sandbox.
    pub fn add_mapping(&mut self, name: impl Into<String>, env_name: impl Into<String>) {
        let name = name.into();
        let sentinel = Self::sentinel_for(&name);
        let env_name = env_name.into();

        self.mappings.push(SentinelMapping {
            name,
            sentinel,
            env_name,
        });
    }

    /// Get all sentinel mappings.
    pub fn mappings(&self) -> &[SentinelMapping] {
        &self.mappings
    }

    /// Inject sentinel values into a sandbox config's environment.
    ///
    /// For each mapped credential, sets the sandbox env var to the
    /// sentinel value instead of the real credential. The sandboxed
    /// process sees `API_KEY=__CRUSTYCLAW_SENTINEL_api_key__`.
    pub fn inject_sentinels(&self, mut config: super::SandboxConfig) -> super::SandboxConfig {
        for mapping in &self.mappings {
            config
                .env
                .insert(mapping.env_name.clone(), mapping.sentinel.clone());
        }
        config
    }

    /// Build a map of sentinel → real value using the provided SecretStore.
    ///
    /// This is used at proxy time (outside the sandbox) to replace
    /// sentinel values in outbound requests with real credentials.
    pub fn resolve_sentinels(
        &self,
        store: &crate::secrets::SecretStore,
    ) -> Result<HashMap<String, String>, IsolationError> {
        let mut resolved = HashMap::new();

        for mapping in &self.mappings {
            let entry = store.get(&mapping.name).ok_or_else(|| {
                IsolationError::CredentialProxy(format!(
                    "credential '{}' not found in secret store",
                    mapping.name
                ))
            })?;

            resolved.insert(mapping.sentinel.clone(), entry.value.expose().to_string());
        }

        Ok(resolved)
    }

    /// Replace sentinel values in a string with real credentials.
    ///
    /// Used to transform outbound request bodies/headers from the
    /// sandbox before forwarding to external APIs.
    pub fn replace_sentinels(&self, text: &str, resolved: &HashMap<String, String>) -> String {
        let mut result = text.to_string();
        for (sentinel, real_value) in resolved {
            result = result.replace(sentinel, real_value);
        }
        result
    }

    /// Check if a string contains any sentinel values.
    ///
    /// Useful for detecting credential leakage in sandbox output.
    pub fn contains_sentinels(&self, text: &str) -> Vec<&str> {
        self.mappings
            .iter()
            .filter(|m| text.contains(&m.sentinel))
            .map(|m| m.name.as_str())
            .collect()
    }

    /// Number of credential mappings.
    pub fn len(&self) -> usize {
        self.mappings.len()
    }

    /// Whether the proxy has any credential mappings.
    pub fn is_empty(&self) -> bool {
        self.mappings.is_empty()
    }
}

impl Default for CredentialProxy {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isolation::SandboxConfig;
    use crate::secrets::{InjectionMethod, SecretEntry, SecretSource, SecretStore, SecretValue};

    #[test]
    fn test_sentinel_generation() {
        assert_eq!(
            CredentialProxy::sentinel_for("api_key"),
            "__CRUSTYCLAW_SENTINEL_api_key__"
        );
        assert_eq!(
            CredentialProxy::sentinel_for("db_password"),
            "__CRUSTYCLAW_SENTINEL_db_password__"
        );
    }

    #[test]
    fn test_add_mapping() {
        let mut proxy = CredentialProxy::new();
        proxy.add_mapping("api_key", "API_KEY");
        proxy.add_mapping("db_pass", "DB_PASSWORD");

        assert_eq!(proxy.len(), 2);
        assert!(!proxy.is_empty());

        assert_eq!(proxy.mappings()[0].name, "api_key");
        assert_eq!(proxy.mappings()[0].env_name, "API_KEY");
        assert_eq!(
            proxy.mappings()[0].sentinel,
            "__CRUSTYCLAW_SENTINEL_api_key__"
        );
    }

    #[test]
    fn test_inject_sentinels() {
        let mut proxy = CredentialProxy::new();
        proxy.add_mapping("api_key", "API_KEY");
        proxy.add_mapping("db_pass", "DB_PASSWORD");

        let config = SandboxConfig::new("sentinel-test").with_workdir("/tmp");
        let injected = proxy.inject_sentinels(config);

        assert_eq!(
            injected.env.get("API_KEY").unwrap(),
            "__CRUSTYCLAW_SENTINEL_api_key__"
        );
        assert_eq!(
            injected.env.get("DB_PASSWORD").unwrap(),
            "__CRUSTYCLAW_SENTINEL_db_pass__"
        );
    }

    #[test]
    fn test_resolve_sentinels() {
        let mut proxy = CredentialProxy::new();
        proxy.add_mapping("api_key", "API_KEY");

        let mut store = SecretStore::new();
        store
            .insert(
                SecretEntry {
                    name: "api_key".to_string(),
                    value: SecretValue::new("sk-real-secret-123"),
                    injection: InjectionMethod::Env("API_KEY".to_string()),
                    description: String::new(),
                },
                SecretSource::Config,
            )
            .unwrap();

        let resolved = proxy.resolve_sentinels(&store).unwrap();
        assert_eq!(
            resolved.get("__CRUSTYCLAW_SENTINEL_api_key__").unwrap(),
            "sk-real-secret-123"
        );
    }

    #[test]
    fn test_resolve_sentinels_missing() {
        let mut proxy = CredentialProxy::new();
        proxy.add_mapping("nonexistent", "NOPE");

        let store = SecretStore::new();
        let result = proxy.resolve_sentinels(&store);
        assert!(result.is_err());
    }

    #[test]
    fn test_replace_sentinels() {
        let mut proxy = CredentialProxy::new();
        proxy.add_mapping("api_key", "API_KEY");

        let mut resolved = HashMap::new();
        resolved.insert(
            "__CRUSTYCLAW_SENTINEL_api_key__".to_string(),
            "sk-real-key".to_string(),
        );

        let input = "Authorization: Bearer __CRUSTYCLAW_SENTINEL_api_key__";
        let output = proxy.replace_sentinels(input, &resolved);
        assert_eq!(output, "Authorization: Bearer sk-real-key");
        assert!(!output.contains("SENTINEL"));
    }

    #[test]
    fn test_contains_sentinels() {
        let mut proxy = CredentialProxy::new();
        proxy.add_mapping("api_key", "API_KEY");
        proxy.add_mapping("db_pass", "DB_PASSWORD");

        let text = "my key is __CRUSTYCLAW_SENTINEL_api_key__ and nothing else";
        let found = proxy.contains_sentinels(text);
        assert_eq!(found, vec!["api_key"]);

        let clean = "no sentinels here";
        let found = proxy.contains_sentinels(clean);
        assert!(found.is_empty());
    }

    #[test]
    fn test_empty_proxy() {
        let proxy = CredentialProxy::new();
        assert!(proxy.is_empty());
        assert_eq!(proxy.len(), 0);
    }
}

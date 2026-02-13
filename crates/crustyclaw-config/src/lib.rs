#![deny(unsafe_code)]

//! Configuration loading, validation, and policy engine for CrustyClaw.
//!
//! Loads TOML configuration files and validates them against expected schemas.
//! Provides the [`AppConfig`] type as the central configuration structure,
//! and the [`policy`] module for role-based access control.

/// Role-based access control policy engine.
pub mod policy;

use std::path::Path;

use serde::{Deserialize, Serialize};

/// Errors that can occur during configuration loading and validation.
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("failed to read config file: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to parse TOML: {0}")]
    Parse(#[from] toml::de::Error),

    #[error("validation error: {0}")]
    Validation(String),
}

/// Top-level application configuration.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    /// Daemon configuration.
    #[serde(default)]
    pub daemon: DaemonConfig,

    /// Signal channel configuration.
    #[serde(default)]
    pub signal: SignalConfig,

    /// Logging configuration.
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Security policy rules loaded from config.
    #[serde(default)]
    pub policy: PolicyConfig,

    /// Isolation / sandbox configuration.
    #[serde(default)]
    pub isolation: IsolationConfig,

    /// Secrets management configuration.
    #[serde(default)]
    pub secrets: SecretsConfig,

    /// Authentication configuration.
    #[serde(default)]
    pub auth: AuthConfig,
}

/// Security policy rules that can be defined in TOML.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct PolicyConfig {
    /// Default effect when no rule matches ("deny" or "allow").
    #[serde(default = "default_policy_default")]
    pub default_effect: String,

    /// Policy rules.
    #[serde(default)]
    pub rules: Vec<PolicyRuleConfig>,
}

/// A single policy rule as expressed in TOML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyRuleConfig {
    /// Role (e.g. "admin", "user", "*").
    pub role: String,
    /// Action (e.g. "read", "write", "*").
    pub action: String,
    /// Resource (e.g. "config", "skills", "*").
    pub resource: String,
    /// Effect ("allow" or "deny").
    pub effect: String,
    /// Priority (higher = evaluated first).
    #[serde(default)]
    pub priority: u32,
}

fn default_policy_default() -> String {
    "deny".to_string()
}

/// Isolation / sandbox configuration.
///
/// Controls how skill commands are isolated. Mirrors Apple's
/// Virtualization.framework configuration model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationConfig {
    /// Isolation backend: "auto", "apple-vz", "linux-ns", or "noop".
    #[serde(default = "default_isolation_backend")]
    pub backend: String,

    /// Default memory limit per sandbox in bytes.
    #[serde(default = "default_isolation_memory")]
    pub default_memory_bytes: u64,

    /// Default CPU fraction per sandbox (0.0–1.0).
    #[serde(default = "default_isolation_cpu_fraction")]
    pub default_cpu_fraction: f64,

    /// Default execution timeout in seconds (0 = no timeout).
    #[serde(default = "default_isolation_timeout_secs")]
    pub default_timeout_secs: u64,

    /// Default network policy: "none", "host-only", "outbound-only".
    #[serde(default = "default_isolation_network")]
    pub default_network: String,

    /// Maximum number of concurrent sandboxes.
    #[serde(default = "default_isolation_max_concurrent")]
    pub max_concurrent: usize,
}

impl Default for IsolationConfig {
    fn default() -> Self {
        Self {
            backend: default_isolation_backend(),
            default_memory_bytes: default_isolation_memory(),
            default_cpu_fraction: default_isolation_cpu_fraction(),
            default_timeout_secs: default_isolation_timeout_secs(),
            default_network: default_isolation_network(),
            max_concurrent: default_isolation_max_concurrent(),
        }
    }
}

fn default_isolation_backend() -> String {
    "auto".to_string()
}

fn default_isolation_memory() -> u64 {
    256 * 1024 * 1024 // 256 MiB
}

fn default_isolation_cpu_fraction() -> f64 {
    0.5
}

fn default_isolation_timeout_secs() -> u64 {
    60
}

fn default_isolation_network() -> String {
    "none".to_string()
}

fn default_isolation_max_concurrent() -> usize {
    4
}

/// Configuration for the core daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Address the daemon listens on for control-plane connections.
    #[serde(default = "default_listen_addr")]
    pub listen_addr: String,

    /// Port the daemon listens on.
    #[serde(default = "default_listen_port")]
    pub listen_port: u16,
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            listen_addr: default_listen_addr(),
            listen_port: default_listen_port(),
        }
    }
}

fn default_listen_addr() -> String {
    "127.0.0.1".to_string()
}

fn default_listen_port() -> u16 {
    9100
}

/// Configuration for the Signal channel adapter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalConfig {
    /// Whether the Signal channel is enabled.
    #[serde(default)]
    pub enabled: bool,

    /// Path to the Signal data directory.
    #[serde(default = "default_signal_data_dir")]
    pub data_dir: String,
}

impl Default for SignalConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            data_dir: default_signal_data_dir(),
        }
    }
}

fn default_signal_data_dir() -> String {
    "data/signal".to_string()
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level filter (e.g. "info", "debug", "trace").
    #[serde(default = "default_log_level")]
    pub level: String,
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
        }
    }
}

fn default_log_level() -> String {
    "info".to_string()
}

/// Secrets management configuration.
///
/// Secrets can be defined inline, loaded from environment variables, or
/// loaded from files. Each secret specifies how it should be injected
/// into sandbox containers (as env vars, files, or both).
///
/// ## TOML Example
///
/// ```toml
/// [secrets]
/// staging_dir = "/run/crustyclaw/secrets"
///
/// [[secrets.entries]]
/// name = "llm_api_key"
/// source = "env"
/// env_var = "CRUSTYCLAW_SECRET_LLM_API_KEY"
/// inject_as = "env"
/// inject_env = "API_KEY"
///
/// [[secrets.entries]]
/// name = "tls_cert"
/// source = "file"
/// file_path = "/etc/crustyclaw/tls.pem"
/// inject_as = "file"
/// inject_path = "/run/secrets/tls.pem"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretsConfig {
    /// Directory used to stage secret files before bind-mounting into containers.
    /// Must be on a tmpfs or encrypted filesystem for production use.
    #[serde(default = "default_secrets_staging_dir")]
    pub staging_dir: String,

    /// Named secret entries.
    #[serde(default)]
    pub entries: Vec<SecretEntryConfig>,
}

impl Default for SecretsConfig {
    fn default() -> Self {
        Self {
            staging_dir: default_secrets_staging_dir(),
            entries: Vec::new(),
        }
    }
}

fn default_secrets_staging_dir() -> String {
    "/run/crustyclaw/secrets".to_string()
}

/// A single secret entry in the configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecretEntryConfig {
    /// Unique name for this secret (used as lookup key).
    pub name: String,

    /// Source of the secret value: "env", "file", or "inline".
    #[serde(default = "default_secret_source")]
    pub source: String,

    /// Environment variable to read from (when source = "env").
    /// Defaults to `CRUSTYCLAW_SECRET_<NAME>` (uppercased).
    #[serde(default)]
    pub env_var: Option<String>,

    /// File path to read from (when source = "file").
    #[serde(default)]
    pub file_path: Option<String>,

    /// Inline value (when source = "inline"). Avoid in production.
    #[serde(default)]
    pub value: Option<String>,

    /// How to inject: "env", "file", or "both".
    #[serde(default = "default_inject_as")]
    pub inject_as: String,

    /// Environment variable name inside the container (when inject_as = "env" or "both").
    #[serde(default)]
    pub inject_env: Option<String>,

    /// File path inside the container (when inject_as = "file" or "both").
    #[serde(default)]
    pub inject_path: Option<String>,

    /// Optional description for operator reference.
    #[serde(default)]
    pub description: String,
}

fn default_secret_source() -> String {
    "env".to_string()
}

fn default_inject_as() -> String {
    "env".to_string()
}

/// Authentication configuration.
///
/// Controls how CLI/TUI sessions are authenticated. The default mode
/// is "local" — the OS identity of the calling process is used as the
/// credential, with no user interaction required.
///
/// ## TOML Example
///
/// ```toml
/// [auth]
/// mode = "local"
///
/// [auth.role_map]
/// alice = "admin"
/// bob = "operator"
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthConfig {
    /// Authentication mode: "local" (OS identity) or "token" (session token file).
    #[serde(default = "default_auth_mode")]
    pub mode: String,

    /// Optional mapping of OS usernames to policy roles.
    /// If a username is not in this map, the default role from
    /// `LocalIdentity::default_role()` is used.
    #[serde(default)]
    pub role_map: std::collections::HashMap<String, String>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            mode: default_auth_mode(),
            role_map: std::collections::HashMap::new(),
        }
    }
}

fn default_auth_mode() -> String {
    "local".to_string()
}

impl AppConfig {
    /// Load configuration from a TOML file at the given path using async I/O.
    pub async fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = tokio::fs::read_to_string(path).await?;
        let config: AppConfig = toml::from_str(&content)?;
        config.validate()?;
        Ok(config)
    }

    /// Parse configuration from a TOML string.
    pub fn parse(s: &str) -> Result<Self, ConfigError> {
        let config: AppConfig = toml::from_str(s)?;
        config.validate()?;
        Ok(config)
    }

    /// Validate the configuration.
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.daemon.listen_port == 0 {
            return Err(ConfigError::Validation(
                "daemon.listen_port must be non-zero".to_string(),
            ));
        }
        if self.daemon.listen_addr.is_empty() {
            return Err(ConfigError::Validation(
                "daemon.listen_addr must not be empty".to_string(),
            ));
        }
        // Validate isolation config
        let valid_backends = ["auto", "apple-vz", "linux-ns", "noop"];
        if !valid_backends.contains(&self.isolation.backend.as_str()) {
            return Err(ConfigError::Validation(format!(
                "isolation.backend must be one of {:?}, got {:?}",
                valid_backends, self.isolation.backend
            )));
        }
        if self.isolation.default_cpu_fraction <= 0.0 || self.isolation.default_cpu_fraction > 1.0 {
            return Err(ConfigError::Validation(format!(
                "isolation.default_cpu_fraction must be in (0.0, 1.0], got {}",
                self.isolation.default_cpu_fraction
            )));
        }
        if self.isolation.default_memory_bytes == 0 {
            return Err(ConfigError::Validation(
                "isolation.default_memory_bytes must be non-zero".to_string(),
            ));
        }
        let valid_networks = ["none", "host-only", "outbound-only"];
        if !valid_networks.contains(&self.isolation.default_network.as_str()) {
            return Err(ConfigError::Validation(format!(
                "isolation.default_network must be one of {:?}, got {:?}",
                valid_networks, self.isolation.default_network
            )));
        }
        if self.isolation.max_concurrent == 0 {
            return Err(ConfigError::Validation(
                "isolation.max_concurrent must be at least 1".to_string(),
            ));
        }

        // Validate policy rules
        for (i, rule) in self.policy.rules.iter().enumerate() {
            if rule.effect != "allow" && rule.effect != "deny" {
                return Err(ConfigError::Validation(format!(
                    "policy.rules[{i}].effect must be \"allow\" or \"deny\", got {:?}",
                    rule.effect
                )));
            }
            if rule.role.is_empty() {
                return Err(ConfigError::Validation(format!(
                    "policy.rules[{i}].role must not be empty"
                )));
            }
        }

        // Validate secrets config
        for (i, entry) in self.secrets.entries.iter().enumerate() {
            if entry.name.is_empty() {
                return Err(ConfigError::Validation(format!(
                    "secrets.entries[{i}].name must not be empty"
                )));
            }
            let valid_sources = ["env", "file", "inline"];
            if !valid_sources.contains(&entry.source.as_str()) {
                return Err(ConfigError::Validation(format!(
                    "secrets.entries[{i}].source must be one of {:?}, got {:?}",
                    valid_sources, entry.source
                )));
            }
            let valid_inject = ["env", "file", "both"];
            if !valid_inject.contains(&entry.inject_as.as_str()) {
                return Err(ConfigError::Validation(format!(
                    "secrets.entries[{i}].inject_as must be one of {:?}, got {:?}",
                    valid_inject, entry.inject_as
                )));
            }
            // Require inject_env when injecting as env
            if (entry.inject_as == "env" || entry.inject_as == "both") && entry.inject_env.is_none()
            {
                return Err(ConfigError::Validation(format!(
                    "secrets.entries[{i}].inject_env is required when inject_as is \"{}\"",
                    entry.inject_as
                )));
            }
            // Require inject_path when injecting as file
            if (entry.inject_as == "file" || entry.inject_as == "both")
                && entry.inject_path.is_none()
            {
                return Err(ConfigError::Validation(format!(
                    "secrets.entries[{i}].inject_path is required when inject_as is \"{}\"",
                    entry.inject_as
                )));
            }
            // Require file_path when source is file
            if entry.source == "file" && entry.file_path.is_none() {
                return Err(ConfigError::Validation(format!(
                    "secrets.entries[{i}].file_path is required when source is \"file\""
                )));
            }
        }

        // Validate auth config
        let valid_auth_modes = ["local", "token"];
        if !valid_auth_modes.contains(&self.auth.mode.as_str()) {
            return Err(ConfigError::Validation(format!(
                "auth.mode must be one of {:?}, got {:?}",
                valid_auth_modes, self.auth.mode
            )));
        }

        Ok(())
    }

    /// Build a [`PolicyEngine`](policy::PolicyEngine) from the loaded policy config.
    pub fn build_policy_engine(&self) -> policy::PolicyEngine {
        let rules: Vec<policy::PolicyRule> = self
            .policy
            .rules
            .iter()
            .map(|r| {
                let effect = if r.effect == "allow" {
                    policy::Effect::Allow
                } else {
                    policy::Effect::Deny
                };
                policy::PolicyRule {
                    role: r.role.clone(),
                    action: r.action.clone(),
                    resource: r.resource.clone(),
                    effect,
                    priority: r.priority,
                }
            })
            .collect();

        let mut engine = policy::build_policy(rules);

        // Add default deny/allow rule at lowest priority
        if self.policy.default_effect == "allow" {
            engine.add_rule(policy::PolicyRule::allow("*", "*", "*").with_priority(0));
        }

        engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = AppConfig::default();
        assert_eq!(config.daemon.listen_addr, "127.0.0.1");
        assert_eq!(config.daemon.listen_port, 9100);
        assert!(!config.signal.enabled);
        assert_eq!(config.logging.level, "info");
    }

    #[test]
    fn test_parse_minimal_toml() {
        let toml = "";
        let config = AppConfig::parse(toml).unwrap();
        assert_eq!(config.daemon.listen_port, 9100);
    }

    #[test]
    fn test_parse_full_toml() {
        let toml = r#"
            [daemon]
            listen_addr = "0.0.0.0"
            listen_port = 8080

            [signal]
            enabled = true
            data_dir = "/var/lib/crustyclaw/signal"

            [logging]
            level = "debug"
        "#;
        let config = AppConfig::parse(toml).unwrap();
        assert_eq!(config.daemon.listen_addr, "0.0.0.0");
        assert_eq!(config.daemon.listen_port, 8080);
        assert!(config.signal.enabled);
        assert_eq!(config.logging.level, "debug");
    }

    #[test]
    fn test_validation_rejects_zero_port() {
        let toml = r#"
            [daemon]
            listen_port = 0
        "#;
        let result = AppConfig::parse(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_rejects_empty_addr() {
        let toml = r#"
            [daemon]
            listen_addr = ""
        "#;
        let result = AppConfig::parse(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_config_from_toml() {
        let toml = r#"
            [policy]
            default_effect = "deny"

            [[policy.rules]]
            role = "admin"
            action = "*"
            resource = "*"
            effect = "allow"
            priority = 10

            [[policy.rules]]
            role = "user"
            action = "read"
            resource = "config"
            effect = "allow"
            priority = 5
        "#;
        let config = AppConfig::parse(toml).unwrap();
        assert_eq!(config.policy.rules.len(), 2);
        assert_eq!(config.policy.default_effect, "deny");

        let mut engine = config.build_policy_engine();
        assert!(engine.is_allowed("admin", "write", "secrets"));
        assert!(engine.is_allowed("user", "read", "config"));
        assert!(!engine.is_allowed("user", "write", "config"));
    }

    #[test]
    fn test_policy_validation_rejects_bad_effect() {
        let toml = r#"
            [[policy.rules]]
            role = "admin"
            action = "*"
            resource = "*"
            effect = "maybe"
        "#;
        let result = AppConfig::parse(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_policy_default_allow() {
        let toml = r#"
            [policy]
            default_effect = "allow"
        "#;
        let config = AppConfig::parse(toml).unwrap();
        let mut engine = config.build_policy_engine();
        // With default allow, everything is permitted
        assert!(engine.is_allowed("anyone", "anything", "anywhere"));
    }

    // ── Secrets config ──────────────────────────────────────────────

    #[test]
    fn test_secrets_config_from_toml() {
        let toml = r#"
            [secrets]
            staging_dir = "/tmp/secrets"

            [[secrets.entries]]
            name = "api_key"
            source = "env"
            inject_as = "env"
            inject_env = "API_KEY"
            description = "LLM API key"

            [[secrets.entries]]
            name = "tls_cert"
            source = "file"
            file_path = "/etc/crustyclaw/tls.pem"
            inject_as = "file"
            inject_path = "/run/secrets/tls.pem"
        "#;
        let config = AppConfig::parse(toml).unwrap();
        assert_eq!(config.secrets.staging_dir, "/tmp/secrets");
        assert_eq!(config.secrets.entries.len(), 2);
        assert_eq!(config.secrets.entries[0].name, "api_key");
        assert_eq!(config.secrets.entries[0].source, "env");
        assert_eq!(config.secrets.entries[1].inject_as, "file");
    }

    #[test]
    fn test_secrets_validation_rejects_empty_name() {
        let toml = r#"
            [[secrets.entries]]
            name = ""
            source = "env"
            inject_as = "env"
            inject_env = "KEY"
        "#;
        let result = AppConfig::parse(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_secrets_validation_rejects_bad_source() {
        let toml = r#"
            [[secrets.entries]]
            name = "key"
            source = "database"
            inject_as = "env"
            inject_env = "KEY"
        "#;
        let result = AppConfig::parse(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_secrets_validation_requires_inject_env() {
        let toml = r#"
            [[secrets.entries]]
            name = "key"
            source = "env"
            inject_as = "env"
        "#;
        let result = AppConfig::parse(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_secrets_validation_requires_inject_path() {
        let toml = r#"
            [[secrets.entries]]
            name = "cert"
            source = "file"
            file_path = "/etc/cert.pem"
            inject_as = "file"
        "#;
        let result = AppConfig::parse(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_secrets_validation_requires_file_path() {
        let toml = r#"
            [[secrets.entries]]
            name = "cert"
            source = "file"
            inject_as = "env"
            inject_env = "CERT"
        "#;
        let result = AppConfig::parse(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_secrets_both_injection() {
        let toml = r#"
            [[secrets.entries]]
            name = "dual_key"
            source = "inline"
            value = "secret-value"
            inject_as = "both"
            inject_env = "DUAL_KEY"
            inject_path = "/run/secrets/dual_key"
        "#;
        let config = AppConfig::parse(toml).unwrap();
        assert_eq!(config.secrets.entries[0].inject_as, "both");
        assert_eq!(
            config.secrets.entries[0].inject_env.as_ref().unwrap(),
            "DUAL_KEY"
        );
        assert_eq!(
            config.secrets.entries[0].inject_path.as_ref().unwrap(),
            "/run/secrets/dual_key"
        );
    }

    // ── Auth config ───────────────────────────────────────────────────

    #[test]
    fn test_auth_config_default() {
        let config = AppConfig::default();
        assert_eq!(config.auth.mode, "local");
        assert!(config.auth.role_map.is_empty());
    }

    #[test]
    fn test_auth_config_from_toml() {
        let toml = r#"
            [auth]
            mode = "local"

            [auth.role_map]
            alice = "admin"
            bob = "operator"
        "#;
        let config = AppConfig::parse(toml).unwrap();
        assert_eq!(config.auth.mode, "local");
        assert_eq!(config.auth.role_map.get("alice").unwrap(), "admin");
        assert_eq!(config.auth.role_map.get("bob").unwrap(), "operator");
    }

    #[test]
    fn test_auth_validation_rejects_bad_mode() {
        let toml = r#"
            [auth]
            mode = "oauth"
        "#;
        let result = AppConfig::parse(toml);
        assert!(result.is_err());
    }

    // ── Async file-based loading ──────────────────────────────────────

    #[tokio::test]
    async fn test_load_from_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("crustyclaw.toml");
        tokio::fs::write(
            &path,
            b"[daemon]\nlisten_port = 4242\nlisten_addr = \"0.0.0.0\"\n",
        )
        .await
        .unwrap();

        let config = AppConfig::load(&path).await.unwrap();
        assert_eq!(config.daemon.listen_port, 4242);
        assert_eq!(config.daemon.listen_addr, "0.0.0.0");
    }

    #[tokio::test]
    async fn test_load_nonexistent_file() {
        let result = AppConfig::load(Path::new("/nonexistent/file.toml")).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_load_invalid_toml_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("bad.toml");
        tokio::fs::write(&path, b"not valid toml [[[")
            .await
            .unwrap();

        let result = AppConfig::load(&path).await;
        assert!(result.is_err());
    }

    // ── Error display ─────────────────────────────────────────────────

    #[test]
    fn test_config_error_display() {
        let err = ConfigError::Validation("bad value".to_string());
        assert_eq!(err.to_string(), "validation error: bad value");
    }

    // ── Isolation validation ──────────────────────────────────────────

    #[test]
    fn test_validation_rejects_invalid_backend() {
        let toml = r#"
            [isolation]
            backend = "docker"
        "#;
        let result = AppConfig::parse(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_rejects_zero_memory() {
        let toml = r#"
            [isolation]
            default_memory_bytes = 0
        "#;
        let result = AppConfig::parse(toml);
        assert!(result.is_err());
    }

    #[test]
    fn test_validation_rejects_bad_cpu_fraction() {
        let toml = r#"
            [isolation]
            default_cpu_fraction = 0.0
        "#;
        let result = AppConfig::parse(toml);
        assert!(result.is_err());
    }
}

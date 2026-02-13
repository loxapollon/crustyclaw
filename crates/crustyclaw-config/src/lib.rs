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

impl AppConfig {
    /// Load configuration from a TOML file at the given path.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
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
}

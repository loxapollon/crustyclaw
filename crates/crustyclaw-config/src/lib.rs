#![deny(unsafe_code)]

//! Configuration loading, validation, and policy engine for CrustyClaw.
//!
//! Loads TOML configuration files and validates them against expected schemas.
//! Provides the [`AppConfig`] type as the central configuration structure.

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
        Ok(())
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
}

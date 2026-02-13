//! Daemon test helpers.
//!
//! Helpers for constructing [`Daemon`] instances in tests with sensible
//! defaults and temporary config files.

use std::path::PathBuf;

use crustyclaw_config::AppConfig;
use crustyclaw_core::Daemon;
use tempfile::TempDir;

/// A test-scoped daemon with an owned temp directory for config files.
///
/// The temp directory is deleted automatically when this value is dropped,
/// guaranteeing cleanup even on panic.
pub struct TestDaemon {
    pub daemon: Daemon,
    pub config_path: PathBuf,
    _temp_dir: TempDir,
}

impl TestDaemon {
    /// Create a daemon backed by a temporary config file containing the given
    /// TOML string.
    pub async fn with_toml(toml_content: &str) -> Self {
        let temp_dir = TempDir::new().expect("failed to create temp dir");
        let config_path = temp_dir.path().join("crustyclaw.toml");
        tokio::fs::write(&config_path, toml_content)
            .await
            .expect("failed to write test config");

        let config = AppConfig::load(&config_path)
            .await
            .expect("failed to parse test config");

        let daemon = Daemon::with_config_path(config, config_path.clone());

        Self {
            daemon,
            config_path,
            _temp_dir: temp_dir,
        }
    }

    /// Create a daemon with default config in a temp directory.
    pub async fn default_config() -> Self {
        Self::with_toml("").await
    }

    /// Overwrite the temp config file with new content (for reload testing).
    pub async fn write_config(&self, toml_content: &str) {
        tokio::fs::write(&self.config_path, toml_content)
            .await
            .expect("failed to write updated config");
    }
}

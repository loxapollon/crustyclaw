//! Configuration builders for tests.
//!
//! Use [`TestConfigBuilder`] to create customised [`AppConfig`] values without
//! repeating boilerplate across crate boundaries.

use crustyclaw_config::AppConfig;

/// Fluent builder for [`AppConfig`] in tests.
///
/// # Example
///
/// ```ignore
/// let config = TestConfigBuilder::new()
///     .listen_port(8080)
///     .listen_addr("0.0.0.0")
///     .build();
/// ```
pub struct TestConfigBuilder {
    config: AppConfig,
}

impl TestConfigBuilder {
    pub fn new() -> Self {
        Self {
            config: AppConfig::default(),
        }
    }

    pub fn listen_addr(mut self, addr: &str) -> Self {
        self.config.daemon.listen_addr = addr.to_string();
        self
    }

    pub fn listen_port(mut self, port: u16) -> Self {
        self.config.daemon.listen_port = port;
        self
    }

    pub fn log_level(mut self, level: &str) -> Self {
        self.config.logging.level = level.to_string();
        self
    }

    pub fn signal_enabled(mut self, enabled: bool) -> Self {
        self.config.signal.enabled = enabled;
        self
    }

    pub fn isolation_backend(mut self, backend: &str) -> Self {
        self.config.isolation.backend = backend.to_string();
        self
    }

    pub fn max_concurrent_sandboxes(mut self, n: usize) -> Self {
        self.config.isolation.max_concurrent = n;
        self
    }

    pub fn build(self) -> AppConfig {
        self.config
    }
}

impl Default for TestConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

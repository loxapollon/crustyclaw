//! Core daemon process — startup, shutdown, and main event loop.

use tokio::sync::broadcast;
use tracing::{info, warn};

use crustyclaw_config::AppConfig;

use crate::message::Envelope;

/// Shutdown signal sent via broadcast channel.
#[derive(Debug, Clone)]
pub struct ShutdownSignal;

/// The main CrustyClaw daemon.
pub struct Daemon {
    config: AppConfig,
    shutdown_tx: broadcast::Sender<ShutdownSignal>,
    _shutdown_rx: broadcast::Receiver<ShutdownSignal>,
    message_tx: broadcast::Sender<Envelope>,
    _message_rx: broadcast::Receiver<Envelope>,
}

impl Daemon {
    /// Create a new daemon instance with the given configuration.
    pub fn new(config: AppConfig) -> Self {
        let (shutdown_tx, _shutdown_rx) = broadcast::channel(1);
        let (message_tx, _message_rx) = broadcast::channel(256);

        Self {
            config,
            shutdown_tx,
            _shutdown_rx,
            message_tx,
            _message_rx,
        }
    }

    /// Run the daemon until a shutdown signal is received.
    pub async fn run(&self) -> Result<(), DaemonError> {
        info!(
            addr = %self.config.daemon.listen_addr,
            port = %self.config.daemon.listen_port,
            "CrustyClaw daemon starting"
        );

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        // Main event loop: wait for shutdown signal
        tokio::select! {
            _ = shutdown_rx.recv() => {
                info!("Shutdown signal received, stopping daemon");
            }
            _ = tokio::signal::ctrl_c() => {
                warn!("Ctrl-C received, initiating graceful shutdown");
                let _ = self.shutdown_tx.send(ShutdownSignal);
            }
        }

        info!("Daemon stopped");
        Ok(())
    }

    /// Request a graceful shutdown of the daemon.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(ShutdownSignal);
    }

    /// Get a sender for the message bus.
    pub fn message_sender(&self) -> broadcast::Sender<Envelope> {
        self.message_tx.clone()
    }

    /// Subscribe to the message bus.
    pub fn message_subscriber(&self) -> broadcast::Receiver<Envelope> {
        self.message_tx.subscribe()
    }

    /// Get a reference to the daemon's configuration.
    pub fn config(&self) -> &AppConfig {
        &self.config
    }
}

/// Errors from the daemon runtime.
#[derive(Debug, thiserror::Error)]
pub enum DaemonError {
    #[error("daemon startup failed: {0}")]
    Startup(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_daemon_creation() {
        let config = AppConfig::default();
        let daemon = Daemon::new(config);
        assert_eq!(daemon.config().daemon.listen_port, 9100);
    }

    #[tokio::test]
    async fn test_daemon_shutdown() {
        let config = AppConfig::default();
        let daemon = Daemon::new(config);

        // Shutdown should not panic
        daemon.shutdown();
    }

    #[tokio::test]
    async fn test_message_bus() {
        let config = AppConfig::default();
        let daemon = Daemon::new(config);

        let tx = daemon.message_sender();
        let mut rx = daemon.message_subscriber();

        let envelope = Envelope::new("test-channel", "Hello, world!");
        tx.send(envelope.clone()).unwrap();

        let received = rx.recv().await.unwrap();
        assert_eq!(received.body, "Hello, world!");
    }
}

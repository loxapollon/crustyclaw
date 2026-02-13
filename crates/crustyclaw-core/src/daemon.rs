//! Core daemon process — startup, shutdown, and main event loop.
//!
//! The daemon handles OS signals for lifecycle management:
//!
//! | Signal | Behaviour |
//! |--------|-----------|
//! | **SIGHUP** | Async-reload config from disk. Published via a `watch` channel so running skills are **never** interrupted — consumers pick up the new config at their next pause / compaction point. |
//! | **SIGTERM** | Initiate graceful shutdown — finish in-flight work, then exit. |
//! | **SIGINT** (Ctrl-C) | Same as SIGTERM. |

use std::path::PathBuf;

use tokio::sync::{broadcast, watch};
use tracing::{error, info, warn};

use crustyclaw_config::AppConfig;

use crate::message::Envelope;

/// Shutdown signal sent via broadcast channel.
#[derive(Debug, Clone)]
pub struct ShutdownSignal;

/// The main CrustyClaw daemon.
pub struct Daemon {
    config: AppConfig,
    config_path: PathBuf,
    config_tx: watch::Sender<AppConfig>,
    config_rx: watch::Receiver<AppConfig>,
    shutdown_tx: broadcast::Sender<ShutdownSignal>,
    _shutdown_rx: broadcast::Receiver<ShutdownSignal>,
    message_tx: broadcast::Sender<Envelope>,
    _message_rx: broadcast::Receiver<Envelope>,
}

impl Daemon {
    /// Create a new daemon instance with the given configuration.
    pub fn new(config: AppConfig) -> Self {
        Self::with_config_path(config, PathBuf::from("crustyclaw.toml"))
    }

    /// Create a new daemon with an explicit config file path for SIGHUP reloads.
    pub fn with_config_path(config: AppConfig, config_path: PathBuf) -> Self {
        let (shutdown_tx, _shutdown_rx) = broadcast::channel(1);
        let (message_tx, _message_rx) = broadcast::channel(256);
        let (config_tx, config_rx) = watch::channel(config.clone());

        Self {
            config,
            config_path,
            config_tx,
            config_rx,
            shutdown_tx,
            _shutdown_rx,
            message_tx,
            _message_rx,
        }
    }

    /// Run the daemon until a shutdown signal is received.
    ///
    /// Listens for OS signals:
    /// - **SIGHUP**: reload configuration from disk (non-interruptive)
    /// - **SIGTERM / SIGINT**: initiate graceful shutdown
    pub async fn run(&self) -> Result<(), DaemonError> {
        info!(
            addr = %self.config.daemon.listen_addr,
            port = %self.config.daemon.listen_port,
            "CrustyClaw daemon starting"
        );

        let mut shutdown_rx = self.shutdown_tx.subscribe();

        #[cfg(unix)]
        {
            use tokio::signal::unix::{SignalKind, signal};

            let mut sighup = signal(SignalKind::hangup()).map_err(DaemonError::Io)?;
            let mut sigterm = signal(SignalKind::terminate()).map_err(DaemonError::Io)?;

            loop {
                tokio::select! {
                    _ = shutdown_rx.recv() => {
                        info!("Shutdown signal received, stopping daemon");
                        break;
                    }
                    _ = tokio::signal::ctrl_c() => {
                        warn!("Ctrl-C received, initiating graceful shutdown");
                        let _ = self.shutdown_tx.send(ShutdownSignal);
                        break;
                    }
                    _ = sigterm.recv() => {
                        warn!("SIGTERM received, initiating graceful shutdown");
                        let _ = self.shutdown_tx.send(ShutdownSignal);
                        break;
                    }
                    _ = sighup.recv() => {
                        info!(path = %self.config_path.display(), "SIGHUP received, reloading config");
                        self.reload_config().await;
                    }
                }
            }
        }

        #[cfg(not(unix))]
        {
            // Non-Unix: only ctrl-c + internal shutdown
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    info!("Shutdown signal received, stopping daemon");
                }
                _ = tokio::signal::ctrl_c() => {
                    warn!("Ctrl-C received, initiating graceful shutdown");
                    let _ = self.shutdown_tx.send(ShutdownSignal);
                }
            }
        }

        info!("Daemon stopped");
        Ok(())
    }

    /// Reload config from disk and publish to watchers.
    ///
    /// This is non-interruptive: the new config is written to a `watch` channel.
    /// Consumers (skill engine, signal service, etc.) observe the update at their
    /// next natural pause / compaction point — running skills are never interrupted.
    async fn reload_config(&self) {
        match AppConfig::load(&self.config_path).await {
            Ok(new_config) => {
                info!("Config reloaded successfully");
                // Publish to all watchers — they pick it up when they're ready,
                // not mid-execution.
                let _ = self.config_tx.send(new_config);
            }
            Err(e) => {
                error!(
                    error = %e,
                    "Config reload failed, keeping current config"
                );
            }
        }
    }

    /// Request a graceful shutdown of the daemon.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(ShutdownSignal);
    }

    /// Subscribe to configuration changes.
    ///
    /// The returned `watch::Receiver` always holds the latest config.
    /// Consumers should call `changed().await` at their own pace — typically
    /// between task iterations or at compaction points — so that running
    /// work is never interrupted by a config reload.
    pub fn config_watcher(&self) -> watch::Receiver<AppConfig> {
        self.config_rx.clone()
    }

    /// Get a sender for the message bus.
    pub fn message_sender(&self) -> broadcast::Sender<Envelope> {
        self.message_tx.clone()
    }

    /// Subscribe to the message bus.
    pub fn message_subscriber(&self) -> broadcast::Receiver<Envelope> {
        self.message_tx.subscribe()
    }

    /// Get a reference to the daemon's current configuration.
    pub fn config(&self) -> &AppConfig {
        &self.config
    }

    /// Get the config file path used for SIGHUP reloads.
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
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
    async fn test_daemon_with_config_path() {
        let config = AppConfig::default();
        let daemon = Daemon::with_config_path(config, PathBuf::from("/etc/crustyclaw.toml"));
        assert_eq!(daemon.config_path(), &PathBuf::from("/etc/crustyclaw.toml"));
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

    #[tokio::test]
    async fn test_config_watcher() {
        let config = AppConfig::default();
        let daemon = Daemon::new(config);

        let rx = daemon.config_watcher();
        assert_eq!(rx.borrow().daemon.listen_port, 9100);
    }

    #[tokio::test]
    async fn test_config_reload_missing_file() {
        let config = AppConfig::default();
        let daemon =
            Daemon::with_config_path(config, PathBuf::from("/nonexistent/crustyclaw.toml"));

        // Should not panic — just logs an error and keeps current config
        daemon.reload_config().await;

        let rx = daemon.config_watcher();
        assert_eq!(rx.borrow().daemon.listen_port, 9100);
    }

    #[tokio::test]
    async fn test_config_reload_valid_file() {
        let dir = std::env::temp_dir().join("crustyclaw-test-reload");
        tokio::fs::create_dir_all(&dir).await.unwrap();
        let path = dir.join("crustyclaw.toml");
        tokio::fs::write(
            &path,
            b"[daemon]\nlisten_addr = \"0.0.0.0\"\nlisten_port = 8080\n",
        )
        .await
        .unwrap();

        let config = AppConfig::default();
        let daemon = Daemon::with_config_path(config, path.clone());

        let mut rx = daemon.config_watcher();
        assert_eq!(rx.borrow().daemon.listen_port, 9100);

        daemon.reload_config().await;

        // Watch channel should have the new config
        rx.changed().await.unwrap();
        assert_eq!(rx.borrow().daemon.listen_port, 8080);
        assert_eq!(rx.borrow().daemon.listen_addr, "0.0.0.0");

        tokio::fs::remove_dir_all(&dir).await.ok();
    }
}

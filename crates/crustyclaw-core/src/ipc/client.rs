//! IPC client — connects to the daemon over a Unix domain socket.
//!
//! Provides a typed client for CLI and TUI to query daemon status,
//! request shutdown, evaluate policies, and inspect runtime state.
//! Uses `hyper` for proper HTTP/1.1 over the Unix socket.

use std::path::PathBuf;

use hyper::body::Bytes;
use hyper_util::rt::TokioIo;
use tokio::net::UnixStream;
use tracing::debug;

use super::types::*;

/// Errors from the IPC client.
#[derive(Debug, thiserror::Error)]
pub enum IpcClientError {
    #[error("failed to connect to daemon socket at {path}: {source}")]
    Connect {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("daemon is not running (socket not found at {0})")]
    NotRunning(PathBuf),

    #[error("request failed: {0}")]
    Request(String),

    #[error("failed to parse response: {0}")]
    Parse(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("daemon returned error: {0}")]
    DaemonError(String),
}

/// Client for communicating with the CrustyClaw daemon via Unix socket.
pub struct IpcClient {
    socket_path: PathBuf,
}

impl IpcClient {
    /// Create a new IPC client targeting the given socket path.
    pub fn new(socket_path: impl Into<PathBuf>) -> Self {
        Self {
            socket_path: socket_path.into(),
        }
    }

    /// Check if the daemon socket exists (daemon is likely running).
    pub fn daemon_available(&self) -> bool {
        self.socket_path.exists()
    }

    /// Send an HTTP request over the Unix socket using hyper and return the response body.
    async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<&[u8]>,
    ) -> Result<Bytes, IpcClientError> {
        if !self.daemon_available() {
            return Err(IpcClientError::NotRunning(self.socket_path.clone()));
        }

        let stream =
            UnixStream::connect(&self.socket_path)
                .await
                .map_err(|e| IpcClientError::Connect {
                    path: self.socket_path.clone(),
                    source: e,
                })?;

        let io = TokioIo::new(stream);

        let (mut sender, conn) =
            hyper::client::conn::http1::handshake::<_, http_body_util::Full<Bytes>>(io)
                .await
                .map_err(|e| IpcClientError::Request(format!("HTTP handshake failed: {e}")))?;

        // Drive the connection in the background
        tokio::spawn(async move {
            if let Err(e) = conn.await {
                tracing::warn!(error = %e, "IPC connection error");
            }
        });

        debug!(method, path, "IPC request");

        let http_method = method
            .parse::<hyper::Method>()
            .map_err(|e| IpcClientError::Request(format!("invalid method: {e}")))?;

        let req_body = if let Some(data) = body {
            http_body_util::Full::new(Bytes::copy_from_slice(data))
        } else {
            http_body_util::Full::new(Bytes::new())
        };

        let mut builder = hyper::Request::builder()
            .method(http_method)
            .uri(path)
            .header("host", "localhost");

        if body.is_some() {
            builder = builder.header("content-type", "application/json");
        }

        let req = builder
            .body(req_body)
            .map_err(|e| IpcClientError::Request(format!("failed to build request: {e}")))?;

        let resp = sender
            .send_request(req)
            .await
            .map_err(|e| IpcClientError::Request(format!("request failed: {e}")))?;

        let status = resp.status();

        let resp_body = http_body_util::BodyExt::collect(resp.into_body())
            .await
            .map_err(|e| IpcClientError::Request(format!("failed to read response body: {e}")))?
            .to_bytes();

        if !status.is_success() {
            if let Ok(err) = serde_json::from_slice::<ErrorResponse>(&resp_body) {
                return Err(IpcClientError::DaemonError(err.error));
            }
            return Err(IpcClientError::Request(format!(
                "unexpected status: {status}"
            )));
        }

        Ok(resp_body)
    }

    // ── Typed API methods ──────────────────────────────────────────────

    /// Health check — is the daemon running and responsive?
    pub async fn health(&self) -> Result<HealthResponse, IpcClientError> {
        let body = self.request("GET", "/health", None).await?;
        serde_json::from_slice(&body).map_err(|e| IpcClientError::Parse(format!("health: {e}")))
    }

    /// Get daemon status.
    pub async fn status(&self) -> Result<StatusResponse, IpcClientError> {
        let body = self.request("GET", "/status", None).await?;
        serde_json::from_slice(&body).map_err(|e| IpcClientError::Parse(format!("status: {e}")))
    }

    /// Request daemon shutdown.
    pub async fn stop(&self) -> Result<StopResponse, IpcClientError> {
        let body = self.request("POST", "/stop", None).await?;
        serde_json::from_slice(&body).map_err(|e| IpcClientError::Parse(format!("stop: {e}")))
    }

    /// Get the daemon's current config as TOML.
    pub async fn config(&self) -> Result<ConfigResponse, IpcClientError> {
        let body = self.request("GET", "/config", None).await?;
        serde_json::from_slice(&body).map_err(|e| IpcClientError::Parse(format!("config: {e}")))
    }

    /// Evaluate a policy rule against the running daemon's policy engine.
    pub async fn policy_eval(
        &self,
        role: &str,
        action: &str,
        resource: &str,
    ) -> Result<PolicyEvalResponse, IpcClientError> {
        let req = PolicyEvalRequest {
            role: role.to_string(),
            action: action.to_string(),
            resource: resource.to_string(),
        };
        let body_bytes = serde_json::to_vec(&req)
            .map_err(|e| IpcClientError::Parse(format!("failed to serialize request: {e}")))?;
        let body = self
            .request("POST", "/policy/evaluate", Some(&body_bytes))
            .await?;
        serde_json::from_slice(&body)
            .map_err(|e| IpcClientError::Parse(format!("policy_eval: {e}")))
    }

    /// List registered plugins.
    pub async fn plugins(&self) -> Result<PluginsResponse, IpcClientError> {
        let body = self.request("GET", "/plugins", None).await?;
        serde_json::from_slice(&body).map_err(|e| IpcClientError::Parse(format!("plugins: {e}")))
    }

    /// List registered skills.
    pub async fn skills(&self) -> Result<SkillsResponse, IpcClientError> {
        let body = self.request("GET", "/skills", None).await?;
        serde_json::from_slice(&body).map_err(|e| IpcClientError::Parse(format!("skills: {e}")))
    }

    /// Get isolation backend status.
    pub async fn isolation(&self) -> Result<IsolationStatusResponse, IpcClientError> {
        let body = self.request("GET", "/isolation", None).await?;
        serde_json::from_slice(&body).map_err(|e| IpcClientError::Parse(format!("isolation: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = IpcClient::new("/tmp/test-crustyclaw.sock");
        assert!(!client.daemon_available()); // socket doesn't exist
    }

    #[tokio::test]
    async fn test_client_not_running_error() {
        let client = IpcClient::new("/tmp/nonexistent-crustyclaw.sock");
        let result = client.health().await;
        assert!(matches!(result, Err(IpcClientError::NotRunning(_))));
    }

    #[tokio::test]
    async fn test_integration_server_client() {
        use crate::plugin::PluginRegistry;
        use crate::skill::SkillRegistry;
        use std::sync::Arc;
        use std::time::Instant;
        use tokio::sync::{broadcast, watch};

        use super::super::server;

        let config = crustyclaw_config::AppConfig::default();
        let (shutdown_tx, _) = broadcast::channel(1);
        let (_, config_rx) = watch::channel(config);

        let state = Arc::new(server::IpcState {
            config: config_rx,
            shutdown_tx: shutdown_tx.clone(),
            skills: Arc::new(SkillRegistry::new()),
            plugins: Arc::new(PluginRegistry::new()),
            started_at: Instant::now(),
        });

        // Use a unique socket path for this test
        let sock_path =
            std::env::temp_dir().join(format!("crustyclaw-test-ipc-{}.sock", std::process::id()));

        // Clean up any stale socket
        std::fs::remove_file(&sock_path).ok();

        let sock_path_clone = sock_path.clone();
        let shutdown_rx = shutdown_tx.subscribe();
        let server_handle = tokio::spawn(async move {
            server::serve(&sock_path_clone, state, shutdown_rx)
                .await
                .unwrap();
        });

        // Give server time to bind
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;

        // Test client calls
        let client = IpcClient::new(&sock_path);
        assert!(client.daemon_available());

        let health = client.health().await.unwrap();
        assert_eq!(health.status, "ok");

        let status = client.status().await.unwrap();
        assert!(status.running);
        assert_eq!(status.listen_port, 9100);

        let skills = client.skills().await.unwrap();
        assert!(skills.skills.is_empty());

        let plugins = client.plugins().await.unwrap();
        assert!(plugins.plugins.is_empty());

        let isolation = client.isolation().await.unwrap();
        assert!(!isolation.backend.is_empty());

        // Stop the daemon via IPC
        let stop = client.stop().await.unwrap();
        assert!(stop.acknowledged);

        // Wait for server to shut down
        let _ = tokio::time::timeout(std::time::Duration::from_secs(2), server_handle).await;

        // Clean up
        std::fs::remove_file(&sock_path).ok();
    }
}

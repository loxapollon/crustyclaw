//! IPC server — axum HTTP router over a Unix domain socket.
//!
//! The daemon binds a Unix socket and exposes a JSON API for
//! the CLI and TUI to query status, request shutdown, evaluate
//! policies, and inspect runtime state.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::routing::{get, post};
use tokio::net::UnixListener;
use tokio::sync::{broadcast, watch};
use tracing::info;

use crustyclaw_config::AppConfig;

use super::types::*;
use crate::daemon::ShutdownSignal;
use crate::plugin::PluginRegistry;
use crate::skill::SkillRegistry;

/// Shared state accessible to all IPC route handlers.
pub struct IpcState {
    pub config: watch::Receiver<AppConfig>,
    pub shutdown_tx: broadcast::Sender<ShutdownSignal>,
    pub skills: Arc<SkillRegistry>,
    pub plugins: Arc<PluginRegistry>,
    pub started_at: Instant,
}

/// Default Unix socket path for daemon IPC.
pub const DEFAULT_SOCKET_PATH: &str = "/tmp/crustyclaw.sock";

/// Build the axum router with all IPC routes.
pub fn router(state: Arc<IpcState>) -> axum::Router {
    axum::Router::new()
        .route("/health", get(handle_health))
        .route("/status", get(handle_status))
        .route("/stop", post(handle_stop))
        .route("/config", get(handle_config))
        .route("/policy/evaluate", post(handle_policy_eval))
        .route("/plugins", get(handle_plugins))
        .route("/skills", get(handle_skills))
        .route("/isolation", get(handle_isolation))
        .with_state(state)
}

/// Start the IPC server on the given Unix socket path.
///
/// Removes any stale socket file before binding. Runs until the
/// shutdown signal is received.
pub async fn serve(
    socket_path: &Path,
    state: Arc<IpcState>,
    mut shutdown_rx: broadcast::Receiver<ShutdownSignal>,
) -> Result<(), std::io::Error> {
    // Remove stale socket file if it exists
    if socket_path.exists() {
        std::fs::remove_file(socket_path)?;
    }

    // Ensure parent directory exists
    if let Some(parent) = socket_path.parent() {
        std::fs::create_dir_all(parent).ok();
    }

    let listener = UnixListener::bind(socket_path)?;
    info!(path = %socket_path.display(), "IPC server listening");

    let app = router(state);

    // Serve with graceful shutdown
    axum::serve(listener, app)
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.recv().await;
            info!("IPC server shutting down");
        })
        .await?;

    // Clean up socket file
    std::fs::remove_file(socket_path).ok();
    Ok(())
}

/// Resolve the socket path from config or use the default.
pub fn socket_path_from_config(config: &AppConfig) -> PathBuf {
    config
        .daemon
        .socket_path
        .as_deref()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_SOCKET_PATH))
}

// ── Route handlers ──────────────────────────────────────────────────────

async fn handle_health(State(state): State<Arc<IpcState>>) -> Json<HealthResponse> {
    let _ = state; // health doesn't need state
    Json(HealthResponse {
        status: "ok".to_string(),
        version: crate::build_info::VERSION.to_string(),
        git_hash: crate::build_info::GIT_HASH.to_string(),
        build_profile: crate::build_info::BUILD_PROFILE.to_string(),
    })
}

async fn handle_status(State(state): State<Arc<IpcState>>) -> Json<StatusResponse> {
    let config = state.config.borrow().clone();
    let uptime = state.started_at.elapsed().as_secs();

    Json(StatusResponse {
        running: true,
        version: crate::build_info::VERSION.to_string(),
        git_hash: crate::build_info::GIT_HASH.to_string(),
        uptime_secs: uptime,
        listen_addr: config.daemon.listen_addr.clone(),
        listen_port: config.daemon.listen_port,
        signal_enabled: config.signal.enabled,
        log_level: config.logging.level.clone(),
        isolation_backend: config.isolation.backend.clone(),
        skills_count: state.skills.list().len(),
        plugins_count: state.plugins.plugin_names().len(),
        pid: std::process::id(),
    })
}

async fn handle_stop(State(state): State<Arc<IpcState>>) -> (StatusCode, Json<StopResponse>) {
    info!("Stop requested via IPC");
    let _ = state.shutdown_tx.send(ShutdownSignal);
    (
        StatusCode::OK,
        Json(StopResponse {
            acknowledged: true,
            message: "Shutdown initiated".to_string(),
        }),
    )
}

async fn handle_config(
    State(state): State<Arc<IpcState>>,
) -> Result<Json<ConfigResponse>, (StatusCode, Json<ErrorResponse>)> {
    let config = state.config.borrow().clone();
    match toml::to_string_pretty(&config) {
        Ok(toml_str) => Ok(Json(ConfigResponse { toml: toml_str })),
        Err(e) => Err((
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ErrorResponse {
                error: format!("Failed to serialize config: {e}"),
            }),
        )),
    }
}

async fn handle_policy_eval(
    State(state): State<Arc<IpcState>>,
    Json(req): Json<PolicyEvalRequest>,
) -> Json<PolicyEvalResponse> {
    let config = state.config.borrow().clone();
    let mut engine = config.build_policy_engine();
    let decision = engine.evaluate(&req.role, &req.action, &req.resource);
    let decision_str = match decision {
        crustyclaw_config::policy::PolicyDecision::Allowed => "allowed",
        crustyclaw_config::policy::PolicyDecision::Denied => "denied",
        crustyclaw_config::policy::PolicyDecision::NoMatch => "no_match",
    };
    Json(PolicyEvalResponse {
        decision: decision_str.to_string(),
        rule_count: engine.rule_count(),
    })
}

async fn handle_plugins(State(state): State<Arc<IpcState>>) -> Json<PluginsResponse> {
    let names = state.plugins.plugin_names();
    let plugins = names
        .iter()
        .filter_map(|name| {
            state.plugins.get_plugin(name).map(|p| PluginInfo {
                name: p.name.clone(),
                version: p.version.clone(),
                description: p.description.clone(),
            })
        })
        .collect();
    Json(PluginsResponse { plugins })
}

async fn handle_skills(State(state): State<Arc<IpcState>>) -> Json<SkillsResponse> {
    let names = state.skills.list();
    let skills = names
        .iter()
        .filter_map(|name| {
            state.skills.get(name).map(|s| SkillInfo {
                name: s.name().to_string(),
                description: s.description().to_string(),
                isolated: s.isolated(),
            })
        })
        .collect();
    Json(SkillsResponse { skills })
}

async fn handle_isolation(State(state): State<Arc<IpcState>>) -> Json<IsolationStatusResponse> {
    let config = state.config.borrow().clone();
    let iso = &config.isolation;

    let pref = match iso.backend.as_str() {
        "docker" => crate::isolation::BackendPreference::Docker,
        "firecracker" => crate::isolation::BackendPreference::Firecracker,
        "apple-vz" => crate::isolation::BackendPreference::AppleVz,
        "linux-ns" => crate::isolation::BackendPreference::LinuxNamespace,
        "noop" => crate::isolation::BackendPreference::Noop,
        _ => crate::isolation::BackendPreference::Auto,
    };
    let backend = crate::isolation::select_backend(&pref);

    Json(IsolationStatusResponse {
        backend: backend.name().to_string(),
        available: backend.available(),
        memory_mb: iso.default_memory_bytes / (1024 * 1024),
        cpu_fraction: iso.default_cpu_fraction,
        timeout_secs: iso.default_timeout_secs,
        network_policy: iso.default_network.clone(),
        max_concurrent: iso.max_concurrent,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn test_state() -> Arc<IpcState> {
        let config = AppConfig::default();
        let (shutdown_tx, _shutdown_rx) = broadcast::channel(1);
        let (_, config_rx) = watch::channel(config);

        Arc::new(IpcState {
            config: config_rx,
            shutdown_tx,
            skills: Arc::new(SkillRegistry::new()),
            plugins: Arc::new(PluginRegistry::new()),
            started_at: Instant::now(),
        })
    }

    #[tokio::test]
    async fn test_health_endpoint() {
        let app = router(test_state());
        let req = Request::get("/health").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let health: HealthResponse = serde_json::from_slice(&body).unwrap();
        assert_eq!(health.status, "ok");
    }

    #[tokio::test]
    async fn test_status_endpoint() {
        let app = router(test_state());
        let req = Request::get("/status").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let status: StatusResponse = serde_json::from_slice(&body).unwrap();
        assert!(status.running);
        assert_eq!(status.listen_port, 9100);
    }

    #[tokio::test]
    async fn test_stop_endpoint() {
        let state = test_state();
        let mut rx = state.shutdown_tx.subscribe();
        let app = router(state);

        let req = Request::post("/stop").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let stop: StopResponse = serde_json::from_slice(&body).unwrap();
        assert!(stop.acknowledged);

        // Verify shutdown signal was sent
        let signal = rx.try_recv();
        assert!(signal.is_ok());
    }

    #[tokio::test]
    async fn test_config_endpoint() {
        let app = router(test_state());
        let req = Request::get("/config").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let config_resp: ConfigResponse = serde_json::from_slice(&body).unwrap();
        assert!(config_resp.toml.contains("listen_port"));
    }

    #[tokio::test]
    async fn test_policy_eval_endpoint() {
        let app = router(test_state());
        let req_body = PolicyEvalRequest {
            role: "admin".to_string(),
            action: "read".to_string(),
            resource: "config".to_string(),
        };
        let req = Request::post("/policy/evaluate")
            .header("content-type", "application/json")
            .body(Body::from(serde_json::to_string(&req_body).unwrap()))
            .unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let eval: PolicyEvalResponse = serde_json::from_slice(&body).unwrap();
        assert!(!eval.decision.is_empty());
    }

    #[tokio::test]
    async fn test_plugins_endpoint() {
        let app = router(test_state());
        let req = Request::get("/plugins").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let plugins: PluginsResponse = serde_json::from_slice(&body).unwrap();
        assert!(plugins.plugins.is_empty()); // no plugins registered
    }

    #[tokio::test]
    async fn test_skills_endpoint() {
        let app = router(test_state());
        let req = Request::get("/skills").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let skills: SkillsResponse = serde_json::from_slice(&body).unwrap();
        assert!(skills.skills.is_empty());
    }

    #[tokio::test]
    async fn test_isolation_endpoint() {
        let app = router(test_state());
        let req = Request::get("/isolation").body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        assert_eq!(resp.status(), StatusCode::OK);

        let body = axum::body::to_bytes(resp.into_body(), usize::MAX)
            .await
            .unwrap();
        let iso: IsolationStatusResponse = serde_json::from_slice(&body).unwrap();
        assert!(!iso.backend.is_empty());
    }
}

//! Shared request/response types for daemon IPC.
//!
//! These types are serialized as JSON over the Unix domain socket
//! transport. Both the IPC server (daemon) and client (CLI/TUI) use
//! these types.

use serde::{Deserialize, Serialize};

/// Daemon health check response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub git_hash: String,
    pub build_profile: String,
}

/// Daemon runtime status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub running: bool,
    pub version: String,
    pub git_hash: String,
    pub uptime_secs: u64,
    pub listen_addr: String,
    pub listen_port: u16,
    pub signal_enabled: bool,
    pub log_level: String,
    pub isolation_backend: String,
    pub skills_count: usize,
    pub plugins_count: usize,
    pub pid: u32,
}

/// Daemon shutdown response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StopResponse {
    pub acknowledged: bool,
    pub message: String,
}

/// Log entry from the daemon.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

/// Log listing response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsResponse {
    pub entries: Vec<LogEntry>,
    pub total: usize,
}

/// Policy evaluation request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvalRequest {
    pub role: String,
    pub action: String,
    pub resource: String,
}

/// Policy evaluation response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyEvalResponse {
    pub decision: String,
    pub rule_count: usize,
}

/// Plugin info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub description: String,
}

/// Plugin listing response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginsResponse {
    pub plugins: Vec<PluginInfo>,
}

/// Skill info.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub isolated: bool,
}

/// Skill listing response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsResponse {
    pub skills: Vec<SkillInfo>,
}

/// Isolation backend status.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IsolationStatusResponse {
    pub backend: String,
    pub available: bool,
    pub memory_mb: u64,
    pub cpu_fraction: f64,
    pub timeout_secs: u64,
    pub network_policy: String,
    pub max_concurrent: usize,
}

/// Configuration response (serialized TOML).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigResponse {
    pub toml: String,
}

/// Generic error response.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
}

//! Apple Virtualization–style isolation for sandboxed skill execution.
//!
//! Models isolation contexts after Apple's `Virtualization.framework`:
//! each skill invocation runs inside a [`Sandbox`] configured with explicit
//! resource grants — filesystem mounts, memory caps, CPU limits, and network
//! policies. The [`SandboxBackend`] trait abstracts over the host platform so
//! the same sandbox configuration works on macOS (Apple Virtualization
//! Framework) and Linux (namespaces + seccomp + landlock).
//!
//! ## Architecture
//!
//! ```text
//! ┌────────────────────────────────────────────────────┐
//! │                   Skill Engine                     │
//! │  ┌──────────────────────────────────────────────┐  │
//! │  │           SandboxConfig (declarative)        │  │
//! │  │  ┌──────────┐ ┌────────┐ ┌───────────────┐  │  │
//! │  │  │FS mounts │ │ Limits │ │ Network policy│  │  │
//! │  │  └──────────┘ └────────┘ └───────────────┘  │  │
//! │  └──────────────────┬───────────────────────────┘  │
//! │                     │ build()                       │
//! │  ┌──────────────────▼───────────────────────────┐  │
//! │  │           Sandbox (running instance)          │  │
//! │  │                                               │  │
//! │  │  ┌─────────────────────────────────────────┐  │  │
//! │  │  │  SandboxBackend (platform-specific)     │  │  │
//! │  │  │  ┌─────────┐ ┌───────┐ ┌───────────┐   │  │  │
//! │  │  │  │Apple VZ │ │ Linux │ │  No-op    │   │  │  │
//! │  │  │  └─────────┘ └───────┘ └───────────┘   │  │  │
//! │  │  └─────────────────────────────────────────┘  │  │
//! │  └───────────────────────────────────────────────┘  │
//! └────────────────────────────────────────────────────┘
//! ```

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::time::Duration;

/// Errors from sandbox creation and execution.
#[derive(Debug, thiserror::Error)]
pub enum IsolationError {
    #[error("sandbox creation failed: {0}")]
    Create(String),

    #[error("sandbox execution failed: {0}")]
    Execution(String),

    #[error("sandbox timeout after {0:?}")]
    Timeout(Duration),

    #[error("resource limit exceeded: {0}")]
    ResourceLimit(String),

    #[error("filesystem policy violation: {0}")]
    FsViolation(String),

    #[error("network policy violation: {0}")]
    NetViolation(String),

    #[error("unsupported backend on this platform: {0}")]
    UnsupportedBackend(String),
}

// ── Resource limits ─────────────────────────────────────────────────────

/// CPU resource limits for a sandbox.
#[derive(Debug, Clone)]
pub struct CpuLimits {
    /// Maximum number of virtual CPU cores.
    pub max_cores: u32,
    /// CPU time ceiling as a fraction of one core (e.g. 0.5 = 50%).
    pub cpu_fraction: f64,
}

impl Default for CpuLimits {
    fn default() -> Self {
        Self {
            max_cores: 1,
            cpu_fraction: 1.0,
        }
    }
}

/// Memory resource limits for a sandbox.
#[derive(Debug, Clone)]
pub struct MemoryLimits {
    /// Maximum resident memory in bytes.
    pub max_bytes: u64,
    /// Whether to allow swap.
    pub allow_swap: bool,
}

impl Default for MemoryLimits {
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024 * 1024, // 256 MiB
            allow_swap: false,
        }
    }
}

/// Combined resource limits.
#[derive(Debug, Clone, Default)]
pub struct ResourceLimits {
    /// CPU limits.
    pub cpu: CpuLimits,
    /// Memory limits.
    pub memory: MemoryLimits,
    /// Maximum execution wall-clock time.
    pub timeout: Option<Duration>,
    /// Maximum number of open file descriptors.
    pub max_open_files: Option<u64>,
    /// Maximum number of spawned processes/threads.
    pub max_pids: Option<u64>,
}

// ── Filesystem policy ───────────────────────────────────────────────────

/// How a host path is exposed to the sandbox.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MountAccess {
    /// Read-only access.
    ReadOnly,
    /// Read-write access.
    ReadWrite,
}

impl fmt::Display for MountAccess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            MountAccess::ReadOnly => write!(f, "ro"),
            MountAccess::ReadWrite => write!(f, "rw"),
        }
    }
}

/// A filesystem mount shared between host and sandbox.
///
/// Mirrors Apple VZ's `VZSharedDirectory` / `VZVirtioFileSystemDeviceConfiguration`.
#[derive(Debug, Clone)]
pub struct SharedMount {
    /// Path on the host.
    pub host_path: PathBuf,
    /// Mount point inside the sandbox.
    pub guest_path: PathBuf,
    /// Access mode.
    pub access: MountAccess,
}

impl SharedMount {
    /// Create a new read-only shared mount.
    pub fn read_only(host: impl Into<PathBuf>, guest: impl Into<PathBuf>) -> Self {
        Self {
            host_path: host.into(),
            guest_path: guest.into(),
            access: MountAccess::ReadOnly,
        }
    }

    /// Create a new read-write shared mount.
    pub fn read_write(host: impl Into<PathBuf>, guest: impl Into<PathBuf>) -> Self {
        Self {
            host_path: host.into(),
            guest_path: guest.into(),
            access: MountAccess::ReadWrite,
        }
    }
}

// ── Network policy ──────────────────────────────────────────────────────

/// Network isolation policy.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NetworkPolicy {
    /// No network access.
    #[default]
    None,
    /// Host-only loopback (sandbox can reach the host daemon socket).
    HostOnly,
    /// Full outbound network access (no inbound).
    OutboundOnly,
    /// Allow specific CIDR ranges.
    AllowList(Vec<String>),
}

impl fmt::Display for NetworkPolicy {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NetworkPolicy::None => write!(f, "none"),
            NetworkPolicy::HostOnly => write!(f, "host-only"),
            NetworkPolicy::OutboundOnly => write!(f, "outbound-only"),
            NetworkPolicy::AllowList(cidrs) => write!(f, "allow[{}]", cidrs.join(",")),
        }
    }
}

// ── Sandbox configuration ───────────────────────────────────────────────

/// Declarative sandbox configuration.
///
/// Analogous to Apple's `VZVirtualMachineConfiguration` — describes
/// *what* resources the sandbox should have without specifying *how*
/// the platform enforces them. The [`SandboxBackend`] translates this
/// into platform-native isolation primitives.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Human-readable label for this sandbox (used in logs).
    pub label: String,
    /// Resource limits.
    pub limits: ResourceLimits,
    /// Shared filesystem mounts.
    pub mounts: Vec<SharedMount>,
    /// Network isolation policy.
    pub network: NetworkPolicy,
    /// Environment variables available to the sandboxed process.
    pub env: HashMap<String, String>,
    /// Working directory inside the sandbox.
    pub workdir: PathBuf,
}

impl SandboxConfig {
    /// Create a minimal sandbox configuration.
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            limits: ResourceLimits::default(),
            mounts: Vec::new(),
            network: NetworkPolicy::default(),
            env: HashMap::new(),
            workdir: PathBuf::from("/workspace"),
        }
    }

    /// Builder: set resource limits.
    pub fn with_limits(mut self, limits: ResourceLimits) -> Self {
        self.limits = limits;
        self
    }

    /// Builder: add a shared mount.
    pub fn with_mount(mut self, mount: SharedMount) -> Self {
        self.mounts.push(mount);
        self
    }

    /// Builder: set network policy.
    pub fn with_network(mut self, policy: NetworkPolicy) -> Self {
        self.network = policy;
        self
    }

    /// Builder: set an environment variable.
    pub fn with_env(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.env.insert(key.into(), value.into());
        self
    }

    /// Builder: set execution timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.limits.timeout = Some(timeout);
        self
    }

    /// Builder: set memory limit.
    pub fn with_memory_limit(mut self, bytes: u64) -> Self {
        self.limits.memory.max_bytes = bytes;
        self
    }

    /// Builder: set working directory.
    pub fn with_workdir(mut self, path: impl Into<PathBuf>) -> Self {
        self.workdir = path.into();
        self
    }

    /// Validate the configuration for obvious errors.
    pub fn validate(&self) -> Result<(), IsolationError> {
        if self.label.is_empty() {
            return Err(IsolationError::Create(
                "sandbox label must not be empty".to_string(),
            ));
        }
        if self.limits.cpu.cpu_fraction <= 0.0 || self.limits.cpu.cpu_fraction > 1.0 {
            return Err(IsolationError::Create(format!(
                "cpu_fraction must be in (0.0, 1.0], got {}",
                self.limits.cpu.cpu_fraction
            )));
        }
        if self.limits.cpu.max_cores == 0 {
            return Err(IsolationError::Create(
                "max_cores must be at least 1".to_string(),
            ));
        }
        if self.limits.memory.max_bytes == 0 {
            return Err(IsolationError::Create(
                "memory limit must be non-zero".to_string(),
            ));
        }
        // Validate mounts: guest paths must be absolute
        for mount in &self.mounts {
            if !mount.guest_path.is_absolute() {
                return Err(IsolationError::FsViolation(format!(
                    "guest mount path must be absolute: {}",
                    mount.guest_path.display()
                )));
            }
        }
        Ok(())
    }
}

// ── Execution result ────────────────────────────────────────────────────

/// The outcome of a sandboxed skill execution.
#[derive(Debug, Clone)]
pub struct SandboxResult {
    /// Exit code (0 = success).
    pub exit_code: i32,
    /// Captured stdout.
    pub stdout: String,
    /// Captured stderr.
    pub stderr: String,
    /// Wall-clock execution time.
    pub elapsed: Duration,
    /// Peak memory usage in bytes (if measurable).
    pub peak_memory_bytes: Option<u64>,
}

impl SandboxResult {
    /// Whether the sandboxed process exited successfully.
    pub fn success(&self) -> bool {
        self.exit_code == 0
    }
}

// ── Backend trait ───────────────────────────────────────────────────────

/// Platform-specific isolation backend.
///
/// Implementations translate a [`SandboxConfig`] into native isolation
/// primitives. Mirrors the role that Apple's Virtualization.framework
/// plays on macOS.
pub trait SandboxBackend: Send + Sync {
    /// Human-readable name of this backend (e.g. "apple-vz", "linux-ns").
    fn name(&self) -> &str;

    /// Whether this backend is available on the current platform.
    fn available(&self) -> bool;

    /// Create and run a sandbox with the given config and command.
    ///
    /// `command` is the argv to execute inside the sandbox.
    fn execute(
        &self,
        config: &SandboxConfig,
        command: &[String],
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<SandboxResult, IsolationError>> + Send + '_>,
    >;
}

// ── Apple Virtualization backend (macOS) ────────────────────────────────

/// Apple Virtualization Framework backend.
///
/// On macOS, uses `Virtualization.framework` to create lightweight VMs for
/// skill isolation. Each sandbox is a minimal Linux VM image booted via
/// `VZLinuxBootLoader` with shared directories exposed as virtio-fs mounts.
///
/// On non-macOS platforms, [`available()`](SandboxBackend::available) returns `false`.
pub struct AppleVzBackend {
    /// Path to the Linux kernel image used for VMs.
    pub kernel_path: PathBuf,
    /// Path to the initrd (initial ramdisk).
    pub initrd_path: PathBuf,
}

impl AppleVzBackend {
    /// Create a new Apple VZ backend with paths to boot assets.
    pub fn new(kernel_path: impl Into<PathBuf>, initrd_path: impl Into<PathBuf>) -> Self {
        Self {
            kernel_path: kernel_path.into(),
            initrd_path: initrd_path.into(),
        }
    }
}

impl SandboxBackend for AppleVzBackend {
    fn name(&self) -> &str {
        "apple-vz"
    }

    fn available(&self) -> bool {
        cfg!(target_os = "macos") && self.kernel_path.exists() && self.initrd_path.exists()
    }

    fn execute(
        &self,
        config: &SandboxConfig,
        command: &[String],
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<SandboxResult, IsolationError>> + Send + '_>,
    > {
        let label = config.label.clone();
        let _timeout = config.limits.timeout;
        let cmd = command.to_vec();

        Box::pin(async move {
            tracing::info!(
                backend = "apple-vz",
                label = %label,
                cmd = ?cmd,
                "Creating Apple VZ sandbox"
            );

            // TODO: FFI bridge to Virtualization.framework
            // - VZVirtualMachineConfiguration
            // - VZLinuxBootLoader with kernel + initrd
            // - VZVirtioFileSystemDeviceConfiguration for shared mounts
            // - VZNetworkDeviceConfiguration based on NetworkPolicy
            // - VZMemoryBalloonDeviceConfiguration for memory limits
            Err(IsolationError::UnsupportedBackend(
                "Apple Virtualization FFI not yet implemented; \
                 requires macOS 12+ and Virtualization.framework bridge"
                    .to_string(),
            ))
        })
    }
}

// ── Linux namespace backend ─────────────────────────────────────────────

/// Linux isolation backend using namespaces, seccomp, and landlock.
///
/// Provides container-grade isolation without requiring a full VM:
///
/// | Mechanism | Purpose |
/// |-----------|---------|
/// | PID namespace | Process tree isolation |
/// | Mount namespace | Filesystem isolation + bind mounts |
/// | Network namespace | Network isolation (veth pair or none) |
/// | User namespace | Unprivileged sandboxing (UID mapping) |
/// | seccomp-BPF | Syscall allowlist |
/// | Landlock | Filesystem access control |
/// | cgroups v2 | Resource limits (CPU, memory, PIDs) |
pub struct LinuxNamespaceBackend {
    /// Seccomp BPF profile (applied when namespace isolation is active).
    pub seccomp_profile: SeccompProfile,
}

/// Seccomp syscall filtering profile.
#[derive(Debug, Clone, Default)]
pub enum SeccompProfile {
    /// Allow all syscalls (no filtering).
    Disabled,
    /// Default restrictive profile: blocks dangerous syscalls.
    #[default]
    Default,
    /// Custom allowlist of permitted syscall names.
    AllowList(Vec<String>),
}

impl LinuxNamespaceBackend {
    /// Create a new Linux namespace backend with the default seccomp profile.
    pub fn new() -> Self {
        Self {
            seccomp_profile: SeccompProfile::Default,
        }
    }

    /// Create with a custom seccomp profile.
    pub fn with_seccomp(profile: SeccompProfile) -> Self {
        Self {
            seccomp_profile: profile,
        }
    }

    /// Generate the cgroup resource limit arguments for a sandbox config.
    fn cgroup_limits(limits: &ResourceLimits) -> Vec<(String, String)> {
        let mut cg = Vec::new();
        // Memory limit
        cg.push((
            "memory.max".to_string(),
            limits.memory.max_bytes.to_string(),
        ));
        if !limits.memory.allow_swap {
            cg.push(("memory.swap.max".to_string(), "0".to_string()));
        }
        // CPU limit (bandwidth controller: quota/period)
        let period_us: u64 = 100_000; // 100ms
        let quota_us = (period_us as f64 * limits.cpu.cpu_fraction) as u64;
        cg.push(("cpu.max".to_string(), format!("{quota_us} {period_us}")));
        // PID limit
        if let Some(max_pids) = limits.max_pids {
            cg.push(("pids.max".to_string(), max_pids.to_string()));
        }
        cg
    }

    /// Generate landlock filesystem rules from the sandbox mounts.
    fn landlock_rules(config: &SandboxConfig) -> Vec<LandlockRule> {
        config
            .mounts
            .iter()
            .map(|m| LandlockRule {
                path: m.host_path.clone(),
                access: match m.access {
                    MountAccess::ReadOnly => LandlockAccess::ReadOnly,
                    MountAccess::ReadWrite => LandlockAccess::ReadWrite,
                },
            })
            .collect()
    }
}

/// A Landlock filesystem access rule.
#[derive(Debug, Clone)]
pub struct LandlockRule {
    /// Host path to grant access to.
    pub path: PathBuf,
    /// Access level.
    pub access: LandlockAccess,
}

/// Landlock access levels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LandlockAccess {
    /// Read-only file access.
    ReadOnly,
    /// Read-write file access.
    ReadWrite,
}

impl Default for LinuxNamespaceBackend {
    fn default() -> Self {
        Self::new()
    }
}

impl SandboxBackend for LinuxNamespaceBackend {
    fn name(&self) -> &str {
        "linux-ns"
    }

    fn available(&self) -> bool {
        cfg!(target_os = "linux")
    }

    fn execute(
        &self,
        config: &SandboxConfig,
        command: &[String],
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<SandboxResult, IsolationError>> + Send + '_>,
    > {
        let label = config.label.clone();
        let _timeout = config.limits.timeout;
        let cgroup_limits = Self::cgroup_limits(&config.limits);
        let landlock_rules = Self::landlock_rules(config);
        let network = config.network.clone();
        let cmd = command.to_vec();

        Box::pin(async move {
            tracing::info!(
                backend = "linux-ns",
                label = %label,
                cmd = ?cmd,
                cgroups = ?cgroup_limits,
                landlock_rules = landlock_rules.len(),
                network = %network,
                "Creating Linux namespace sandbox"
            );

            // TODO: Implement via clone3(CLONE_NEWPID | CLONE_NEWNS | CLONE_NEWNET | CLONE_NEWUSER)
            // 1. Create cgroup and write limits
            // 2. Set up mount namespace with bind mounts
            // 3. Apply Landlock ruleset
            // 4. Install seccomp-BPF filter
            // 5. Set up network namespace (veth or none)
            // 6. exec the command
            Err(IsolationError::UnsupportedBackend(
                "Linux namespace isolation not yet implemented; \
                 requires clone3, seccomp, and landlock syscall integration"
                    .to_string(),
            ))
        })
    }
}

// ── No-op (development) backend ─────────────────────────────────────────

/// No-op sandbox backend for development and testing.
///
/// Runs commands directly on the host with no isolation. Resource limits
/// are logged but not enforced. **Never use in production.**
pub struct NoopBackend;

impl SandboxBackend for NoopBackend {
    fn name(&self) -> &str {
        "noop"
    }

    fn available(&self) -> bool {
        true
    }

    fn execute(
        &self,
        config: &SandboxConfig,
        command: &[String],
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<SandboxResult, IsolationError>> + Send + '_>,
    > {
        let label = config.label.clone();
        let timeout = config.limits.timeout;
        let workdir = config.workdir.clone();
        let env = config.env.clone();
        let cmd = command.to_vec();

        Box::pin(async move {
            tracing::warn!(
                backend = "noop",
                label = %label,
                "Running WITHOUT isolation (development mode)"
            );

            if cmd.is_empty() {
                return Err(IsolationError::Execution(
                    "command must not be empty".to_string(),
                ));
            }

            let start = std::time::Instant::now();

            let mut proc = tokio::process::Command::new(&cmd[0]);
            proc.args(&cmd[1..])
                .current_dir(&workdir)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped());

            for (k, v) in &env {
                proc.env(k, v);
            }

            let child = proc
                .spawn()
                .map_err(|e| IsolationError::Execution(format!("spawn failed: {e}")))?;

            let output = match timeout {
                Some(dur) => {
                    let result = tokio::time::timeout(dur, child.wait_with_output()).await;
                    match result {
                        Ok(Ok(output)) => output,
                        Ok(Err(e)) => {
                            return Err(IsolationError::Execution(format!("wait failed: {e}")));
                        }
                        Err(_) => {
                            return Err(IsolationError::Timeout(dur));
                        }
                    }
                }
                None => child
                    .wait_with_output()
                    .await
                    .map_err(|e| IsolationError::Execution(format!("wait failed: {e}")))?,
            };

            let elapsed = start.elapsed();

            Ok(SandboxResult {
                exit_code: output.status.code().unwrap_or(-1),
                stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
                elapsed,
                peak_memory_bytes: None,
            })
        })
    }
}

// ── Sandbox (high-level handle) ─────────────────────────────────────────

/// A configured sandbox ready to execute skill commands.
///
/// Analogous to an instantiated `VZVirtualMachine` — holds a validated
/// config and a reference to the backend that will enforce isolation.
pub struct Sandbox {
    config: SandboxConfig,
    backend: Box<dyn SandboxBackend>,
}

impl Sandbox {
    /// Create a new sandbox from a config and backend.
    ///
    /// Returns an error if the config is invalid or the backend is
    /// unavailable on this platform.
    pub fn new(
        config: SandboxConfig,
        backend: Box<dyn SandboxBackend>,
    ) -> Result<Self, IsolationError> {
        config.validate()?;
        if !backend.available() {
            return Err(IsolationError::UnsupportedBackend(format!(
                "backend '{}' is not available on this platform",
                backend.name()
            )));
        }
        Ok(Self { config, backend })
    }

    /// Execute a command inside this sandbox.
    pub async fn execute(&self, command: &[String]) -> Result<SandboxResult, IsolationError> {
        if command.is_empty() {
            return Err(IsolationError::Execution(
                "command must not be empty".to_string(),
            ));
        }
        self.backend.execute(&self.config, command).await
    }

    /// Get the sandbox label.
    pub fn label(&self) -> &str {
        &self.config.label
    }

    /// Get the backend name.
    pub fn backend_name(&self) -> &str {
        self.backend.name()
    }

    /// Get a reference to the sandbox config.
    pub fn config(&self) -> &SandboxConfig {
        &self.config
    }
}

// ── Auto-detect backend ─────────────────────────────────────────────────

/// Isolation backend preference.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum BackendPreference {
    /// Auto-detect the best available backend.
    #[default]
    Auto,
    /// Force Apple Virtualization Framework.
    AppleVz,
    /// Force Linux namespace isolation.
    LinuxNamespace,
    /// Force no-op (development only).
    Noop,
}

impl fmt::Display for BackendPreference {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            BackendPreference::Auto => write!(f, "auto"),
            BackendPreference::AppleVz => write!(f, "apple-vz"),
            BackendPreference::LinuxNamespace => write!(f, "linux-ns"),
            BackendPreference::Noop => write!(f, "noop"),
        }
    }
}

/// Select the best available isolation backend for this platform.
///
/// Priority: Apple VZ (macOS) > Linux namespaces (Linux) > No-op.
pub fn select_backend(preference: &BackendPreference) -> Box<dyn SandboxBackend> {
    match preference {
        BackendPreference::AppleVz => Box::new(AppleVzBackend::new(
            "/usr/local/share/crustyclaw/vmlinuz",
            "/usr/local/share/crustyclaw/initrd.img",
        )),
        BackendPreference::LinuxNamespace => Box::new(LinuxNamespaceBackend::new()),
        BackendPreference::Noop => Box::new(NoopBackend),
        BackendPreference::Auto => {
            if cfg!(target_os = "linux") {
                Box::new(LinuxNamespaceBackend::new())
            } else if cfg!(target_os = "macos") {
                Box::new(AppleVzBackend::new(
                    "/usr/local/share/crustyclaw/vmlinuz",
                    "/usr/local/share/crustyclaw/initrd.img",
                ))
            } else {
                tracing::warn!("No native isolation available, falling back to noop backend");
                Box::new(NoopBackend)
            }
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sandbox_config_defaults() {
        let config = SandboxConfig::new("test-skill");
        assert_eq!(config.label, "test-skill");
        assert_eq!(config.limits.cpu.max_cores, 1);
        assert_eq!(config.limits.memory.max_bytes, 256 * 1024 * 1024);
        assert_eq!(config.network, NetworkPolicy::None);
        assert!(config.mounts.is_empty());
        assert!(config.env.is_empty());
    }

    #[test]
    fn test_sandbox_config_builder() {
        let config = SandboxConfig::new("my-skill")
            .with_memory_limit(512 * 1024 * 1024)
            .with_timeout(Duration::from_secs(30))
            .with_network(NetworkPolicy::HostOnly)
            .with_mount(SharedMount::read_only("/opt/skills/my-skill", "/skill"))
            .with_mount(SharedMount::read_write("/tmp/scratch", "/scratch"))
            .with_env("SKILL_NAME", "my-skill")
            .with_workdir("/skill");

        assert_eq!(config.limits.memory.max_bytes, 512 * 1024 * 1024);
        assert_eq!(config.limits.timeout, Some(Duration::from_secs(30)));
        assert_eq!(config.network, NetworkPolicy::HostOnly);
        assert_eq!(config.mounts.len(), 2);
        assert_eq!(config.mounts[0].access, MountAccess::ReadOnly);
        assert_eq!(config.mounts[1].access, MountAccess::ReadWrite);
        assert_eq!(config.env.get("SKILL_NAME").unwrap(), "my-skill");
        assert_eq!(config.workdir, PathBuf::from("/skill"));
    }

    #[test]
    fn test_sandbox_config_validation_ok() {
        let config = SandboxConfig::new("valid")
            .with_mount(SharedMount::read_only("/host/path", "/guest/path"));
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_sandbox_config_validation_empty_label() {
        let config = SandboxConfig::new("");
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_sandbox_config_validation_bad_cpu_fraction() {
        let mut config = SandboxConfig::new("bad-cpu");
        config.limits.cpu.cpu_fraction = 0.0;
        assert!(config.validate().is_err());

        config.limits.cpu.cpu_fraction = 1.5;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_sandbox_config_validation_zero_memory() {
        let mut config = SandboxConfig::new("zero-mem");
        config.limits.memory.max_bytes = 0;
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_sandbox_config_validation_relative_guest_path() {
        let config = SandboxConfig::new("bad-mount")
            .with_mount(SharedMount::read_only("/host/path", "relative/guest"));
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_noop_backend_available() {
        let backend = NoopBackend;
        assert!(backend.available());
        assert_eq!(backend.name(), "noop");
    }

    #[test]
    fn test_linux_ns_backend() {
        let backend = LinuxNamespaceBackend::new();
        assert_eq!(backend.name(), "linux-ns");
        // available() depends on target_os
    }

    #[test]
    fn test_apple_vz_backend() {
        let backend = AppleVzBackend::new("/nonexistent/kernel", "/nonexistent/initrd");
        assert_eq!(backend.name(), "apple-vz");
        // Not available because paths don't exist
        assert!(!backend.available());
    }

    #[test]
    fn test_sandbox_creation_with_noop() {
        let config = SandboxConfig::new("test");
        let sandbox = Sandbox::new(config, Box::new(NoopBackend)).unwrap();
        assert_eq!(sandbox.label(), "test");
        assert_eq!(sandbox.backend_name(), "noop");
    }

    #[test]
    fn test_sandbox_rejects_unavailable_backend() {
        let config = SandboxConfig::new("test");
        let backend = AppleVzBackend::new("/nonexistent", "/nonexistent");
        let result = Sandbox::new(config, Box::new(backend));
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_noop_backend_execute_echo() {
        let config = SandboxConfig::new("echo-test")
            .with_timeout(Duration::from_secs(5))
            .with_workdir("/tmp");

        let sandbox = Sandbox::new(config, Box::new(NoopBackend)).unwrap();
        let result = sandbox
            .execute(&[
                "echo".to_string(),
                "hello".to_string(),
                "sandbox".to_string(),
            ])
            .await
            .unwrap();

        assert!(result.success());
        assert_eq!(result.stdout.trim(), "hello sandbox");
        assert!(result.stderr.is_empty());
    }

    #[tokio::test]
    async fn test_noop_backend_execute_failure() {
        let config = SandboxConfig::new("fail-test").with_workdir("/tmp");

        let sandbox = Sandbox::new(config, Box::new(NoopBackend)).unwrap();
        let result = sandbox.execute(&["false".to_string()]).await.unwrap();

        assert!(!result.success());
        assert_ne!(result.exit_code, 0);
    }

    #[tokio::test]
    async fn test_noop_backend_timeout() {
        let config = SandboxConfig::new("timeout-test")
            .with_timeout(Duration::from_millis(100))
            .with_workdir("/tmp");

        let sandbox = Sandbox::new(config, Box::new(NoopBackend)).unwrap();
        let result = sandbox
            .execute(&["sleep".to_string(), "10".to_string()])
            .await;

        assert!(matches!(result, Err(IsolationError::Timeout(_))));
    }

    #[tokio::test]
    async fn test_noop_backend_env_vars() {
        let config = SandboxConfig::new("env-test")
            .with_env("MY_VAR", "hello_from_sandbox")
            .with_workdir("/tmp");

        let sandbox = Sandbox::new(config, Box::new(NoopBackend)).unwrap();
        let result = sandbox
            .execute(&[
                "sh".to_string(),
                "-c".to_string(),
                "echo $MY_VAR".to_string(),
            ])
            .await
            .unwrap();

        assert!(result.success());
        assert_eq!(result.stdout.trim(), "hello_from_sandbox");
    }

    #[tokio::test]
    async fn test_sandbox_rejects_empty_command() {
        let config = SandboxConfig::new("empty-cmd").with_workdir("/tmp");
        let sandbox = Sandbox::new(config, Box::new(NoopBackend)).unwrap();
        let result = sandbox.execute(&[]).await;
        assert!(result.is_err());
    }

    #[test]
    fn test_select_backend_noop() {
        let backend = select_backend(&BackendPreference::Noop);
        assert_eq!(backend.name(), "noop");
    }

    #[test]
    fn test_select_backend_auto() {
        let backend = select_backend(&BackendPreference::Auto);
        // On Linux CI, should select linux-ns
        if cfg!(target_os = "linux") {
            assert_eq!(backend.name(), "linux-ns");
        }
    }

    #[test]
    fn test_cgroup_limits_generation() {
        let limits = ResourceLimits {
            cpu: CpuLimits {
                max_cores: 2,
                cpu_fraction: 0.5,
            },
            memory: MemoryLimits {
                max_bytes: 128 * 1024 * 1024,
                allow_swap: false,
            },
            timeout: None,
            max_open_files: None,
            max_pids: Some(64),
        };

        let cg = LinuxNamespaceBackend::cgroup_limits(&limits);
        assert!(cg.iter().any(|(k, _)| k == "memory.max"));
        assert!(cg.iter().any(|(k, _)| k == "memory.swap.max"));
        assert!(cg.iter().any(|(k, _)| k == "cpu.max"));
        assert!(cg.iter().any(|(k, _)| k == "pids.max"));
    }

    #[test]
    fn test_landlock_rules_generation() {
        let config = SandboxConfig::new("ll-test")
            .with_mount(SharedMount::read_only("/opt/skills", "/skills"))
            .with_mount(SharedMount::read_write("/tmp/scratch", "/scratch"));

        let rules = LinuxNamespaceBackend::landlock_rules(&config);
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0].access, LandlockAccess::ReadOnly);
        assert_eq!(rules[1].access, LandlockAccess::ReadWrite);
    }

    #[test]
    fn test_network_policy_display() {
        assert_eq!(NetworkPolicy::None.to_string(), "none");
        assert_eq!(NetworkPolicy::HostOnly.to_string(), "host-only");
        assert_eq!(NetworkPolicy::OutboundOnly.to_string(), "outbound-only");
        assert_eq!(
            NetworkPolicy::AllowList(vec!["10.0.0.0/8".to_string()]).to_string(),
            "allow[10.0.0.0/8]"
        );
    }

    #[test]
    fn test_mount_access_display() {
        assert_eq!(MountAccess::ReadOnly.to_string(), "ro");
        assert_eq!(MountAccess::ReadWrite.to_string(), "rw");
    }

    #[test]
    fn test_sandbox_result_success() {
        let result = SandboxResult {
            exit_code: 0,
            stdout: String::new(),
            stderr: String::new(),
            elapsed: Duration::from_millis(100),
            peak_memory_bytes: None,
        };
        assert!(result.success());
    }

    #[test]
    fn test_sandbox_result_failure() {
        let result = SandboxResult {
            exit_code: 1,
            stdout: String::new(),
            stderr: "error".to_string(),
            elapsed: Duration::from_millis(50),
            peak_memory_bytes: Some(1024),
        };
        assert!(!result.success());
    }

    #[test]
    fn test_backend_preference_display() {
        assert_eq!(BackendPreference::Auto.to_string(), "auto");
        assert_eq!(BackendPreference::AppleVz.to_string(), "apple-vz");
        assert_eq!(BackendPreference::LinuxNamespace.to_string(), "linux-ns");
        assert_eq!(BackendPreference::Noop.to_string(), "noop");
    }

    #[test]
    fn test_seccomp_profile_default() {
        let backend = LinuxNamespaceBackend::new();
        assert!(matches!(backend.seccomp_profile, SeccompProfile::Default));
    }

    #[test]
    fn test_seccomp_custom_allowlist() {
        let backend = LinuxNamespaceBackend::with_seccomp(SeccompProfile::AllowList(vec![
            "read".to_string(),
            "write".to_string(),
            "exit_group".to_string(),
        ]));
        if let SeccompProfile::AllowList(syscalls) = &backend.seccomp_profile {
            assert_eq!(syscalls.len(), 3);
        } else {
            panic!("expected AllowList");
        }
    }
}

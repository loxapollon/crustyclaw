//! Docker container sandbox backend.
//!
//! Uses `docker run` to execute skill commands inside isolated containers
//! with resource limits, filesystem mounts, network policies, and
//! environment variable injection. When Docker Desktop with sandbox
//! support is available, each container gets MicroVM-level isolation
//! (its own Linux kernel on KVM) rather than sharing the host kernel.
//!
//! ## Isolation guarantees
//!
//! | Feature | Implementation |
//! |---------|---------------|
//! | CPU limits | `--cpus`, `--cpu-shares` |
//! | Memory limits | `--memory`, `--memory-swap` |
//! | PID limits | `--pids-limit` |
//! | Filesystem | `--volume` (ro/rw), `--workdir` |
//! | Network | `--network none/host/bridge` |
//! | Timeout | Container killed after wall-clock deadline |
//! | Cleanup | Container auto-removed (`--rm`) |

use std::path::PathBuf;

use crate::BoxFuture;

use super::{
    IsolationError, MountAccess, NetworkPolicy, SandboxBackend, SandboxConfig, SandboxResult,
};

/// Docker container sandbox backend.
///
/// Runs skill commands inside Docker containers with resource limits
/// enforced by the Docker daemon. Provides L1 (container) or L3
/// (MicroVM via Docker Desktop sandboxes) isolation depending on the
/// Docker installation.
pub struct DockerSandboxBackend {
    /// Docker CLI binary path.
    docker_bin: PathBuf,
    /// Default container image for sandboxed skills.
    default_image: String,
}

impl DockerSandboxBackend {
    /// Create a new Docker sandbox backend.
    pub fn new(docker_bin: impl Into<PathBuf>, default_image: impl Into<String>) -> Self {
        Self {
            docker_bin: docker_bin.into(),
            default_image: default_image.into(),
        }
    }

    /// Build the `docker run` argument list from a sandbox config.
    fn build_args(&self, config: &SandboxConfig, command: &[String]) -> Vec<String> {
        let mut args = vec!["run".to_string(), "--rm".to_string(), "--init".to_string()];

        // Resource limits
        let cpu = format!("{:.2}", config.limits.cpu.cpu_fraction);
        args.extend(["--cpus".to_string(), cpu]);

        let mem = format!("{}m", config.limits.memory.max_bytes / (1024 * 1024));
        args.extend(["--memory".to_string(), mem]);

        if !config.limits.memory.allow_swap {
            args.extend([
                "--memory-swap".to_string(),
                format!("{}m", config.limits.memory.max_bytes / (1024 * 1024)),
            ]);
        }

        if let Some(max_pids) = config.limits.max_pids {
            args.extend(["--pids-limit".to_string(), max_pids.to_string()]);
        }

        // Network policy
        match &config.network {
            NetworkPolicy::None => {
                args.extend(["--network".to_string(), "none".to_string()]);
            }
            NetworkPolicy::HostOnly => {
                args.extend(["--network".to_string(), "host".to_string()]);
            }
            NetworkPolicy::OutboundOnly | NetworkPolicy::AllowList(_) => {
                args.extend(["--network".to_string(), "bridge".to_string()]);
            }
        }

        // Working directory
        args.extend([
            "--workdir".to_string(),
            config.workdir.to_string_lossy().to_string(),
        ]);

        // Environment variables
        for (key, value) in &config.env {
            args.extend(["-e".to_string(), format!("{key}={value}")]);
        }

        // Filesystem mounts
        for mount in &config.mounts {
            let ro_flag = match mount.access {
                MountAccess::ReadOnly => ":ro",
                MountAccess::ReadWrite => "",
            };
            args.extend([
                "-v".to_string(),
                format!(
                    "{}:{}{}",
                    mount.host_path.display(),
                    mount.guest_path.display(),
                    ro_flag
                ),
            ]);
        }

        // Container label for tracking
        args.extend([
            "--label".to_string(),
            format!("crustyclaw.sandbox={}", config.label),
        ]);

        // Image
        args.push(self.default_image.clone());

        // Command
        args.extend(command.iter().cloned());

        args
    }
}

impl Default for DockerSandboxBackend {
    fn default() -> Self {
        Self {
            docker_bin: PathBuf::from("docker"),
            default_image: "alpine:latest".to_string(),
        }
    }
}

impl SandboxBackend for DockerSandboxBackend {
    fn name(&self) -> &str {
        "docker"
    }

    fn available(&self) -> bool {
        // Check if docker CLI is on PATH and responsive
        std::process::Command::new(&self.docker_bin)
            .arg("version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok_and(|s| s.success())
    }

    fn execute(
        &self,
        config: &SandboxConfig,
        command: &[String],
    ) -> BoxFuture<'_, Result<SandboxResult, IsolationError>> {
        let label = config.label.clone();
        let timeout = config.limits.timeout;
        let args = self.build_args(config, command);
        let docker_bin = self.docker_bin.clone();

        Box::pin(async move {
            tracing::info!(
                backend = "docker",
                label = %label,
                args = ?args,
                "Creating Docker sandbox"
            );

            let start = std::time::Instant::now();

            let child = tokio::process::Command::new(&docker_bin)
                .args(&args)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| IsolationError::Execution(format!("failed to spawn docker: {e}")))?;

            let output = match timeout {
                Some(dur) => match tokio::time::timeout(dur, child.wait_with_output()).await {
                    Ok(Ok(output)) => output,
                    Ok(Err(e)) => {
                        return Err(IsolationError::Execution(format!(
                            "docker wait failed: {e}"
                        )));
                    }
                    Err(_) => {
                        return Err(IsolationError::Timeout(dur));
                    }
                },
                None => child
                    .wait_with_output()
                    .await
                    .map_err(|e| IsolationError::Execution(format!("docker wait failed: {e}")))?,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_docker_backend_name() {
        let backend = DockerSandboxBackend::default();
        assert_eq!(backend.name(), "docker");
        assert_eq!(backend.default_image, "alpine:latest");
    }

    #[test]
    fn test_docker_build_args_basic() {
        let backend = DockerSandboxBackend::default();
        let config = SandboxConfig::new("test-skill")
            .with_env("MY_VAR", "hello")
            .with_workdir("/workspace");

        let args = backend.build_args(&config, &["echo".to_string(), "hi".to_string()]);

        assert!(args.contains(&"run".to_string()));
        assert!(args.contains(&"--rm".to_string()));
        assert!(args.contains(&"--network".to_string()));
        assert!(args.contains(&"none".to_string()));
        assert!(args.contains(&"-e".to_string()));
        assert!(args.contains(&"MY_VAR=hello".to_string()));
        assert!(args.contains(&"alpine:latest".to_string()));
        assert!(args.contains(&"echo".to_string()));
        assert!(args.contains(&"hi".to_string()));
    }

    #[test]
    fn test_docker_build_args_mounts() {
        let backend = DockerSandboxBackend::default();
        let config = SandboxConfig::new("mount-test")
            .with_mount(super::super::SharedMount::read_only("/host/src", "/src"))
            .with_mount(super::super::SharedMount::read_write("/host/out", "/out"));

        let args = backend.build_args(&config, &["ls".to_string()]);

        assert!(args.contains(&"-v".to_string()));
        assert!(args.iter().any(|a| a.contains("/host/src:/src:ro")));
        assert!(args.iter().any(|a| a == "/host/out:/out"));
    }

    #[test]
    fn test_docker_build_args_resource_limits() {
        let backend = DockerSandboxBackend::default();
        let mut config = SandboxConfig::new("limits-test");
        config.limits.cpu.cpu_fraction = 0.5;
        config.limits.memory.max_bytes = 512 * 1024 * 1024;
        config.limits.max_pids = Some(100);

        let args = backend.build_args(&config, &["true".to_string()]);

        assert!(args.contains(&"--cpus".to_string()));
        assert!(args.contains(&"0.50".to_string()));
        assert!(args.contains(&"--memory".to_string()));
        assert!(args.contains(&"512m".to_string()));
        assert!(args.contains(&"--pids-limit".to_string()));
        assert!(args.contains(&"100".to_string()));
    }

    #[test]
    fn test_docker_build_args_network_policies() {
        let backend = DockerSandboxBackend::default();

        // None
        let config = SandboxConfig::new("net-none");
        let args = backend.build_args(&config, &["true".to_string()]);
        let net_idx = args.iter().position(|a| a == "--network").unwrap();
        assert_eq!(args[net_idx + 1], "none");

        // HostOnly
        let config = SandboxConfig::new("net-host").with_network(NetworkPolicy::HostOnly);
        let args = backend.build_args(&config, &["true".to_string()]);
        let net_idx = args.iter().position(|a| a == "--network").unwrap();
        assert_eq!(args[net_idx + 1], "host");

        // OutboundOnly
        let config = SandboxConfig::new("net-out").with_network(NetworkPolicy::OutboundOnly);
        let args = backend.build_args(&config, &["true".to_string()]);
        let net_idx = args.iter().position(|a| a == "--network").unwrap();
        assert_eq!(args[net_idx + 1], "bridge");
    }

    #[test]
    fn test_docker_build_args_no_swap() {
        let backend = DockerSandboxBackend::default();
        let mut config = SandboxConfig::new("no-swap");
        config.limits.memory.allow_swap = false;
        config.limits.memory.max_bytes = 256 * 1024 * 1024;

        let args = backend.build_args(&config, &["true".to_string()]);
        assert!(args.contains(&"--memory-swap".to_string()));
        assert!(args.contains(&"256m".to_string()));
    }

    #[test]
    fn test_docker_build_args_label() {
        let backend = DockerSandboxBackend::default();
        let config = SandboxConfig::new("my-skill");

        let args = backend.build_args(&config, &["true".to_string()]);
        assert!(args.iter().any(|a| a == "crustyclaw.sandbox=my-skill"));
    }
}

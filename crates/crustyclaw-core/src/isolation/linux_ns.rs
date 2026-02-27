//! Linux namespace isolation backend.
//!
//! Provides container-grade isolation without requiring a full VM:
//!
//! | Mechanism | Purpose |
//! |-----------|---------|
//! | PID namespace | Process tree isolation |
//! | Mount namespace | Filesystem isolation + bind mounts |
//! | Network namespace | Network isolation (veth pair or none) |
//! | User namespace | Unprivileged sandboxing (UID mapping) |
//! | seccomp-BPF | Syscall allowlist |
//! | Landlock | Filesystem access control |
//! | cgroups v2 | Resource limits (CPU, memory, PIDs) |

use std::path::PathBuf;

use crate::BoxFuture;

use super::{
    IsolationError, MountAccess, ResourceLimits, SandboxBackend, SandboxConfig, SandboxResult,
};

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

/// Linux isolation backend using namespaces, seccomp, and landlock.
pub struct LinuxNamespaceBackend {
    /// Seccomp BPF profile (applied when namespace isolation is active).
    pub seccomp_profile: SeccompProfile,
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
    pub(crate) fn cgroup_limits(limits: &ResourceLimits) -> Vec<(String, String)> {
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
    pub(crate) fn landlock_rules(config: &SandboxConfig) -> Vec<LandlockRule> {
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
    ) -> BoxFuture<'_, Result<SandboxResult, IsolationError>> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isolation::{CpuLimits, MemoryLimits, SharedMount};

    #[test]
    fn test_linux_ns_backend() {
        let backend = LinuxNamespaceBackend::new();
        assert_eq!(backend.name(), "linux-ns");
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
}

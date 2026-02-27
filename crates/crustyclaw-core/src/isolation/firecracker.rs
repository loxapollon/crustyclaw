//! Firecracker microVM sandbox backend.
//!
//! Uses Firecracker's HTTP API (over a Unix domain socket) to create
//! lightweight microVMs for skill isolation. Each sandbox gets its own
//! Linux kernel running on KVM, providing L3 isolation:
//!
//! - ~125–200 ms boot time
//! - <5 MiB memory overhead per VM
//! - Hard security boundary (guest kernel + hypervisor)
//!
//! Requires:
//! - Linux host with KVM enabled (`/dev/kvm`)
//! - `firecracker` binary on PATH
//! - A root filesystem image and kernel image

use std::path::PathBuf;

use crate::BoxFuture;

use super::{IsolationError, SandboxBackend, SandboxConfig, SandboxResult};

/// Firecracker microVM backend configuration.
pub struct FirecrackerBackend {
    /// Path to the `firecracker` binary.
    pub firecracker_bin: PathBuf,
    /// Path to the Linux kernel image for microVMs.
    pub kernel_image: PathBuf,
    /// Path to the root filesystem image.
    pub rootfs_image: PathBuf,
    /// Base directory for Firecracker API sockets.
    pub socket_dir: PathBuf,
}

impl FirecrackerBackend {
    /// Create a new Firecracker backend with explicit paths.
    pub fn new(
        firecracker_bin: impl Into<PathBuf>,
        kernel_image: impl Into<PathBuf>,
        rootfs_image: impl Into<PathBuf>,
        socket_dir: impl Into<PathBuf>,
    ) -> Self {
        Self {
            firecracker_bin: firecracker_bin.into(),
            kernel_image: kernel_image.into(),
            rootfs_image: rootfs_image.into(),
            socket_dir: socket_dir.into(),
        }
    }

    /// Check if KVM is available on this host.
    fn kvm_available() -> bool {
        std::path::Path::new("/dev/kvm").exists()
    }

    /// Generate the Firecracker VM configuration JSON.
    fn vm_config_json(&self, config: &SandboxConfig) -> String {
        let vcpu_count = config.limits.cpu.max_cores;
        let mem_mib = config.limits.memory.max_bytes / (1024 * 1024);

        format!(
            r#"{{
  "boot-source": {{
    "kernel_image_path": "{}",
    "boot_args": "console=ttyS0 reboot=k panic=1 pci=off"
  }},
  "drives": [
    {{
      "drive_id": "rootfs",
      "path_on_host": "{}",
      "is_root_device": true,
      "is_read_only": true
    }}
  ],
  "machine-config": {{
    "vcpu_count": {},
    "mem_size_mib": {}
  }}
}}"#,
            self.kernel_image.display(),
            self.rootfs_image.display(),
            vcpu_count,
            mem_mib,
        )
    }
}

impl Default for FirecrackerBackend {
    fn default() -> Self {
        Self {
            firecracker_bin: PathBuf::from("firecracker"),
            kernel_image: PathBuf::from("/usr/local/share/crustyclaw/vmlinux"),
            rootfs_image: PathBuf::from("/usr/local/share/crustyclaw/rootfs.ext4"),
            socket_dir: PathBuf::from("/run/crustyclaw/firecracker"),
        }
    }
}

impl SandboxBackend for FirecrackerBackend {
    fn name(&self) -> &str {
        "firecracker"
    }

    fn available(&self) -> bool {
        cfg!(target_os = "linux")
            && Self::kvm_available()
            && std::process::Command::new(&self.firecracker_bin)
                .arg("--version")
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
        let _timeout = config.limits.timeout;
        let cmd = command.to_vec();
        let vm_config = self.vm_config_json(config);

        Box::pin(async move {
            tracing::info!(
                backend = "firecracker",
                label = %label,
                cmd = ?cmd,
                "Creating Firecracker microVM sandbox"
            );

            tracing::debug!(vm_config = %vm_config, "Firecracker VM configuration");

            // TODO: Implement Firecracker lifecycle:
            // 1. Create Unix socket for API
            // 2. Spawn firecracker --api-sock <socket>
            // 3. PUT /machine-config (vcpu, mem)
            // 4. PUT /boot-source (kernel, boot_args)
            // 5. PUT /drives/rootfs (root filesystem)
            // 6. PUT /network-interfaces (based on NetworkPolicy)
            // 7. PUT /actions {"action_type": "InstanceStart"}
            // 8. Execute command inside VM via vsock or serial
            // 9. Collect stdout/stderr
            // 10. PUT /actions {"action_type": "SendCtrlAltDel"}
            // 11. Cleanup socket and resources
            Err(IsolationError::UnsupportedBackend(
                "Firecracker microVM integration not yet implemented; \
                 requires KVM, firecracker binary, and kernel/rootfs images"
                    .to_string(),
            ))
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isolation::SandboxConfig;

    #[test]
    fn test_firecracker_backend_name() {
        let backend = FirecrackerBackend::default();
        assert_eq!(backend.name(), "firecracker");
    }

    #[test]
    fn test_firecracker_default_paths() {
        let backend = FirecrackerBackend::default();
        assert_eq!(backend.firecracker_bin, PathBuf::from("firecracker"));
        assert_eq!(
            backend.kernel_image,
            PathBuf::from("/usr/local/share/crustyclaw/vmlinux")
        );
        assert_eq!(
            backend.rootfs_image,
            PathBuf::from("/usr/local/share/crustyclaw/rootfs.ext4")
        );
    }

    #[test]
    fn test_firecracker_vm_config_json() {
        let backend = FirecrackerBackend::default();
        let mut config = SandboxConfig::new("test-vm");
        config.limits.cpu.max_cores = 2;
        config.limits.memory.max_bytes = 512 * 1024 * 1024;

        let json = backend.vm_config_json(&config);
        assert!(json.contains("\"vcpu_count\": 2"));
        assert!(json.contains("\"mem_size_mib\": 512"));
        assert!(json.contains("vmlinux"));
        assert!(json.contains("rootfs.ext4"));
    }

    #[test]
    fn test_firecracker_not_available_without_kvm() {
        let backend = FirecrackerBackend::new(
            "/nonexistent/firecracker",
            "/nonexistent/kernel",
            "/nonexistent/rootfs",
            "/nonexistent/sockets",
        );
        // Should not be available — binary doesn't exist
        assert!(!backend.available());
    }
}

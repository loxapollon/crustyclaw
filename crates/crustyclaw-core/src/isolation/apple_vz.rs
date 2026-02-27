//! Apple Virtualization Framework backend (macOS).
//!
//! On macOS, uses `Virtualization.framework` to create lightweight VMs for
//! skill isolation. Each sandbox is a minimal Linux VM image booted via
//! `VZLinuxBootLoader` with shared directories exposed as virtio-fs mounts.
//!
//! On non-macOS platforms, [`available()`](super::SandboxBackend::available)
//! returns `false`.

use std::path::PathBuf;

use crate::BoxFuture;

use super::{IsolationError, SandboxBackend, SandboxConfig, SandboxResult};

/// Apple Virtualization Framework backend.
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
    ) -> BoxFuture<'_, Result<SandboxResult, IsolationError>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apple_vz_backend() {
        let backend = AppleVzBackend::new("/nonexistent/kernel", "/nonexistent/initrd");
        assert_eq!(backend.name(), "apple-vz");
        // Not available because paths don't exist
        assert!(!backend.available());
    }
}

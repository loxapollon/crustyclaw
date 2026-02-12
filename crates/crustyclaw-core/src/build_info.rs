//! Build-time metadata embedded by the build script.
//!
//! Provides version, git commit hash, and build timestamp for use in
//! status displays, logging, and diagnostics.

/// The git commit hash at build time (short form).
pub const GIT_HASH: &str = env!("CRUSTYCLAW_GIT_HASH");

/// The build timestamp as a Unix epoch string.
pub const BUILD_TIMESTAMP: &str = env!("CRUSTYCLAW_BUILD_TIMESTAMP");

/// The build profile (`debug` or `release`).
pub const BUILD_PROFILE: &str = env!("CRUSTYCLAW_BUILD_PROFILE");

/// The crate version from Cargo.toml.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Return a formatted version string including git hash and profile.
///
/// Example: `"0.1.0 (abc1234, debug)"`
pub fn version_string() -> String {
    format!("{VERSION} ({GIT_HASH}, {BUILD_PROFILE})")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_string_not_empty() {
        let v = version_string();
        assert!(!v.is_empty());
        assert!(v.contains(VERSION));
    }

    #[test]
    fn test_git_hash_not_empty() {
        assert!(!GIT_HASH.is_empty());
    }

    #[test]
    fn test_build_profile() {
        // In test mode, profile is "debug"
        assert_eq!(BUILD_PROFILE, "debug");
    }
}

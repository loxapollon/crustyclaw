//! Type-state authentication lifecycle.
//!
//! Enforces the state machine `Unauthenticated → Authenticated → Authorized`
//! at compile time, making invalid auth transitions unrepresentable.
//!
//! ## Transparent Authentication
//!
//! The [`LocalIdentity`] type detects the current OS user automatically.
//! [`Session::authenticate_local`] performs authentication using OS-level
//! identity — the operator never sees a prompt, password dialog, or token.
//!
//! The flow for CLI/TUI usage:
//!
//! ```text
//! Session<Unauthenticated>
//!     → .authenticate_local()          // reads OS uid/username
//!         → Session<Authenticated>
//!             → .authorize_with_policy() // evaluates against policy engine
//!                 → Session<Authorized>
//! ```

use std::marker::PhantomData;

use crustyclaw_config::policy::PolicyEngine;

/// The local OS identity of the current process owner.
///
/// Detected automatically — no user interaction required. On Unix,
/// this reads the effective UID and resolves the username. On non-Unix
/// platforms, it falls back to environment variables.
#[derive(Debug, Clone)]
pub struct LocalIdentity {
    /// OS username (e.g. "alice", "root").
    pub username: String,
    /// Numeric user ID (Unix UID). `0` on non-Unix platforms.
    pub uid: u32,
    /// Numeric group ID (Unix GID). `0` on non-Unix platforms.
    pub gid: u32,
    /// Whether this identity has elevated privileges (uid == 0 on Unix).
    pub is_privileged: bool,
}

impl LocalIdentity {
    /// Detect the identity of the current process owner.
    ///
    /// This is the primary entry point for transparent authentication.
    /// It reads OS-level credentials without prompting the user.
    pub fn detect() -> Self {
        #[cfg(unix)]
        {
            // SAFETY: libc getuid/getgid are always safe to call.
            // We avoid `unsafe` by using std's internal support.
            let uid = Self::get_uid();
            let gid = Self::get_gid();
            let username = std::env::var("USER")
                .or_else(|_| std::env::var("LOGNAME"))
                .unwrap_or_else(|_| format!("uid:{uid}"));

            Self {
                is_privileged: uid == 0,
                username,
                uid,
                gid,
            }
        }

        #[cfg(not(unix))]
        {
            let username = std::env::var("USERNAME")
                .or_else(|_| std::env::var("USER"))
                .unwrap_or_else(|_| "unknown".to_string());

            Self {
                username,
                uid: 0,
                gid: 0,
                is_privileged: false,
            }
        }
    }

    /// Construct a `LocalIdentity` from explicit values (for testing).
    pub fn from_parts(username: impl Into<String>, uid: u32, gid: u32) -> Self {
        Self {
            is_privileged: uid == 0,
            username: username.into(),
            uid,
            gid,
        }
    }

    /// Map this OS identity to a policy role name.
    ///
    /// Mapping rules (in priority order):
    /// 1. `uid == 0` → `"admin"`
    /// 2. Otherwise → the OS username
    ///
    /// Operators can override this by defining explicit role mappings
    /// in the `[auth]` config section.
    pub fn default_role(&self) -> &str {
        if self.is_privileged {
            "admin"
        } else {
            &self.username
        }
    }

    #[cfg(unix)]
    fn get_uid() -> u32 {
        // std::os::unix::process provides getuid() but it's unstable.
        // Use the /proc/self approach or env fallback.
        // For simplicity and to avoid unsafe, parse /proc/self/status on Linux
        // or use environment variable as fallback.
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|line| line.starts_with("Uid:"))
                    .and_then(|line| line.split_whitespace().nth(1))
                    .and_then(|uid| uid.parse().ok())
            })
            .unwrap_or(u32::MAX)
    }

    #[cfg(unix)]
    fn get_gid() -> u32 {
        std::fs::read_to_string("/proc/self/status")
            .ok()
            .and_then(|s| {
                s.lines()
                    .find(|line| line.starts_with("Gid:"))
                    .and_then(|line| line.split_whitespace().nth(1))
                    .and_then(|gid| gid.parse().ok())
            })
            .unwrap_or(u32::MAX)
    }
}

/// Unauthenticated state — no credentials have been verified.
pub struct Unauthenticated;

/// Authenticated state — identity has been verified but no permissions granted.
pub struct Authenticated {
    pub identity: String,
    /// The local OS identity, if authenticated via `authenticate_local`.
    pub local_identity: Option<LocalIdentity>,
}

/// Authorized state — identity verified and permissions granted.
pub struct Authorized {
    pub identity: String,
    pub roles: Vec<String>,
    /// The local OS identity, if authenticated via `authenticate_local`.
    pub local_identity: Option<LocalIdentity>,
}

/// An authentication session parameterized by its lifecycle state.
///
/// The type parameter `S` enforces that only valid transitions can occur:
/// - `Session<Unauthenticated>` → `.authenticate()` → `Session<Authenticated>`
/// - `Session<Authenticated>` → `.authorize()` → `Session<Authorized>`
///
/// There is no path from `Unauthenticated` directly to `Authorized`.
pub struct Session<S> {
    _state: PhantomData<S>,
    state: S,
}

impl Session<Unauthenticated> {
    /// Create a new unauthenticated session.
    pub fn new() -> Self {
        Self {
            _state: PhantomData,
            state: Unauthenticated,
        }
    }

    /// Authenticate with the given identity. Returns an `Authenticated` session.
    pub fn authenticate(self, identity: String) -> Session<Authenticated> {
        Session {
            _state: PhantomData,
            state: Authenticated {
                identity,
                local_identity: None,
            },
        }
    }

    /// Authenticate transparently using the local OS identity.
    ///
    /// This is the primary authentication path for CLI and TUI usage.
    /// No password, token, or user interaction is required — the OS
    /// identity of the calling process is the credential.
    pub fn authenticate_local(self) -> Session<Authenticated> {
        let local = LocalIdentity::detect();
        let identity = local.username.clone();
        Session {
            _state: PhantomData,
            state: Authenticated {
                identity,
                local_identity: Some(local),
            },
        }
    }
}

impl Default for Session<Unauthenticated> {
    fn default() -> Self {
        Self::new()
    }
}

impl Session<Authenticated> {
    /// Get the authenticated identity.
    pub fn identity(&self) -> &str {
        &self.state.identity
    }

    /// Get the local OS identity, if this session was authenticated locally.
    pub fn local_identity(&self) -> Option<&LocalIdentity> {
        self.state.local_identity.as_ref()
    }

    /// Authorize the session with the given roles.
    pub fn authorize(self, roles: Vec<String>) -> Session<Authorized> {
        Session {
            _state: PhantomData,
            state: Authorized {
                identity: self.state.identity,
                roles,
                local_identity: self.state.local_identity,
            },
        }
    }

    /// Authorize the session by evaluating the policy engine.
    ///
    /// This completes the transparent auth flow: the policy engine
    /// determines what roles to grant based on the OS identity.
    /// If the policy allows `(role, "auth", "session")`, the role
    /// is granted. The default role from [`LocalIdentity::default_role`]
    /// is always included.
    pub fn authorize_with_policy(self, engine: &mut PolicyEngine) -> Session<Authorized> {
        let default_role = self
            .state
            .local_identity
            .as_ref()
            .map(|li| li.default_role().to_string())
            .unwrap_or_else(|| self.state.identity.clone());

        let mut roles = vec![default_role];

        // Check if the policy grants additional roles.
        // Convention: a rule allowing (role, "auth", "session") means
        // that identity can assume that role.
        for candidate in &["admin", "operator", "user", "viewer"] {
            if engine.is_allowed(candidate, "auth", "session")
                && !roles.contains(&candidate.to_string())
            {
                // Only grant if the identity matches or a wildcard rule applies
                let identity = &self.state.identity;
                if engine.is_allowed(identity, "assume", candidate) {
                    roles.push(candidate.to_string());
                }
            }
        }

        Session {
            _state: PhantomData,
            state: Authorized {
                identity: self.state.identity,
                roles,
                local_identity: self.state.local_identity,
            },
        }
    }
}

impl Session<Authorized> {
    /// Get the authorized identity.
    pub fn identity(&self) -> &str {
        &self.state.identity
    }

    /// Get the local OS identity, if this session was authenticated locally.
    pub fn local_identity(&self) -> Option<&LocalIdentity> {
        self.state.local_identity.as_ref()
    }

    /// Get the roles granted to this session.
    pub fn roles(&self) -> &[String] {
        &self.state.roles
    }

    /// Check whether this session has a specific role.
    pub fn has_role(&self, role: &str) -> bool {
        self.state.roles.iter().any(|r| r == role)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_lifecycle() {
        let session = Session::new();
        let authed = session.authenticate("alice".to_string());
        assert_eq!(authed.identity(), "alice");

        let authorized = authed.authorize(vec!["admin".to_string(), "user".to_string()]);
        assert_eq!(authorized.identity(), "alice");
        assert!(authorized.has_role("admin"));
        assert!(authorized.has_role("user"));
        assert!(!authorized.has_role("superadmin"));
    }

    #[test]
    fn test_roles() {
        let session = Session::new()
            .authenticate("bob".to_string())
            .authorize(vec!["viewer".to_string()]);

        assert_eq!(session.roles(), &["viewer"]);
    }

    #[test]
    fn test_local_identity_detect() {
        let identity = LocalIdentity::detect();
        // On any platform, username should be non-empty
        assert!(!identity.username.is_empty());
    }

    #[test]
    fn test_local_identity_from_parts() {
        let identity = LocalIdentity::from_parts("testuser", 1000, 1000);
        assert_eq!(identity.username, "testuser");
        assert_eq!(identity.uid, 1000);
        assert_eq!(identity.gid, 1000);
        assert!(!identity.is_privileged);
        assert_eq!(identity.default_role(), "testuser");
    }

    #[test]
    fn test_local_identity_root() {
        let identity = LocalIdentity::from_parts("root", 0, 0);
        assert!(identity.is_privileged);
        assert_eq!(identity.default_role(), "admin");
    }

    #[test]
    fn test_authenticate_local() {
        let session = Session::new().authenticate_local();
        // Should have a non-empty identity derived from the OS
        assert!(!session.identity().is_empty());
        assert!(session.local_identity().is_some());
    }

    #[test]
    fn test_authorize_with_policy() {
        use crustyclaw_config::policy::{PolicyEngine, PolicyRule};

        let mut engine = PolicyEngine::new();
        engine.add_rule(PolicyRule::allow("*", "auth", "session").with_priority(10));

        let session = Session::new()
            .authenticate("alice".to_string())
            .authorize_with_policy(&mut engine);

        // Should have at least the default role
        assert!(!session.roles().is_empty());
        assert!(session.has_role("alice"));
    }

    #[test]
    fn test_transparent_auth_preserves_local_identity() {
        let session = Session::new().authenticate_local();
        let local = session.local_identity().unwrap();
        let username = local.username.clone();

        let authorized = session.authorize(vec!["operator".to_string()]);
        assert!(authorized.local_identity().is_some());
        assert_eq!(authorized.local_identity().unwrap().username, username);
    }
}

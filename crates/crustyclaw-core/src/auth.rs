//! Type-state authentication lifecycle.
//!
//! Enforces the state machine `Unauthenticated → Authenticated → Authorized`
//! at compile time, making invalid auth transitions unrepresentable.

use std::marker::PhantomData;

/// Unauthenticated state — no credentials have been verified.
pub struct Unauthenticated;

/// Authenticated state — identity has been verified but no permissions granted.
pub struct Authenticated {
    pub identity: String,
}

/// Authorized state — identity verified and permissions granted.
pub struct Authorized {
    pub identity: String,
    pub roles: Vec<String>,
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
            state: Authenticated { identity },
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

    /// Authorize the session with the given roles.
    pub fn authorize(self, roles: Vec<String>) -> Session<Authorized> {
        Session {
            _state: PhantomData,
            state: Authorized {
                identity: self.state.identity,
                roles,
            },
        }
    }
}

impl Session<Authorized> {
    /// Get the authorized identity.
    pub fn identity(&self) -> &str {
        &self.state.identity
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
}

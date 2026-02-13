//! Type-state Signal adapter — enforces `Unlinked → Linked → Verified` lifecycle.
//!
//! State transitions (`link`, `verify`) are async because a real Signal protocol
//! implementation performs network I/O during account linking and verification.

use tracing::info;

use crate::SignalError;

/// Signal adapter session states.
pub mod session {
    /// The adapter has not yet been linked to a Signal account.
    pub struct Unlinked;

    /// The adapter is linked to a Signal account but not yet verified.
    pub struct Linked {
        pub phone_number: String,
    }

    /// The adapter is linked and verified, ready to send/receive messages.
    pub struct Verified {
        pub phone_number: String,
    }
}

/// The Signal channel adapter.
///
/// Generic over its session state `S` to enforce correct lifecycle transitions
/// at compile time. There is no way to construct a `Verified` adapter without
/// first going through `Unlinked → Linked → Verified`.
pub struct SignalAdapter<S> {
    state: S,
}

impl SignalAdapter<session::Unlinked> {
    /// Create a new unlinked Signal adapter.
    pub fn new() -> Self {
        info!("Creating new Signal adapter (unlinked)");
        Self {
            state: session::Unlinked,
        }
    }

    /// Link to a Signal account. Returns the adapter in `Linked` state.
    ///
    /// This is async because the real Signal protocol performs network I/O
    /// during the linking handshake.
    pub async fn link(
        self,
        phone_number: String,
    ) -> Result<SignalAdapter<session::Linked>, SignalError> {
        info!(phone = %phone_number, "Linking Signal account");
        // TODO: actual Signal protocol linking handshake (network I/O)
        Ok(SignalAdapter {
            state: session::Linked { phone_number },
        })
    }
}

impl Default for SignalAdapter<session::Unlinked> {
    fn default() -> Self {
        Self::new()
    }
}

impl SignalAdapter<session::Linked> {
    /// Verify the linked Signal account. Returns the adapter in `Verified` state.
    ///
    /// This is async because the real Signal protocol performs network I/O
    /// during verification (e.g. safety-number exchange).
    pub async fn verify(self) -> Result<SignalAdapter<session::Verified>, SignalError> {
        info!(phone = %self.state.phone_number, "Verifying Signal account");
        // TODO: actual Signal protocol verification (network I/O)
        Ok(SignalAdapter {
            state: session::Verified {
                phone_number: self.state.phone_number,
            },
        })
    }

    /// Get the phone number this adapter is linked to.
    pub fn phone_number(&self) -> &str {
        &self.state.phone_number
    }
}

impl SignalAdapter<session::Verified> {
    /// Get the phone number this adapter is verified for.
    pub fn phone_number(&self) -> &str {
        &self.state.phone_number
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_type_state_lifecycle() {
        let adapter = SignalAdapter::new();
        let linked = adapter.link("+1234567890".to_string()).await.unwrap();
        assert_eq!(linked.phone_number(), "+1234567890");
        let verified = linked.verify().await.unwrap();
        assert_eq!(verified.phone_number(), "+1234567890");
    }

    #[test]
    fn test_default() {
        let _adapter = SignalAdapter::default();
    }
}

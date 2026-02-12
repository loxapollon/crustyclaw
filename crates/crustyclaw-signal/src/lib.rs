#![deny(unsafe_code)]

//! Signal protocol channel adapter for CrustyClaw.
//!
//! This crate provides the bridge between the CrustyClaw daemon and the Signal
//! messaging protocol. Messages received via Signal are routed into the daemon's
//! message bus, and outbound messages are delivered back through Signal.
//!
//! The Signal session follows a type-state lifecycle:
//! `Unlinked → Linked → Verified`

use tracing::info;

/// Signal adapter session states (type-state pattern).
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
/// at compile time.
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
    pub fn link(self, phone_number: String) -> SignalAdapter<session::Linked> {
        info!(phone = %phone_number, "Linking Signal account");
        SignalAdapter {
            state: session::Linked { phone_number },
        }
    }
}

impl Default for SignalAdapter<session::Unlinked> {
    fn default() -> Self {
        Self::new()
    }
}

impl SignalAdapter<session::Linked> {
    /// Verify the linked Signal account. Returns the adapter in `Verified` state.
    pub fn verify(self) -> SignalAdapter<session::Verified> {
        info!(phone = %self.state.phone_number, "Verifying Signal account");
        SignalAdapter {
            state: session::Verified {
                phone_number: self.state.phone_number,
            },
        }
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

/// Errors from the Signal adapter.
#[derive(Debug, thiserror::Error)]
pub enum SignalError {
    #[error("Signal linking failed: {0}")]
    LinkingFailed(String),

    #[error("Signal verification failed: {0}")]
    VerificationFailed(String),

    #[error("message send failed: {0}")]
    SendFailed(String),

    #[error("message receive failed: {0}")]
    ReceiveFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_type_state_lifecycle() {
        let adapter = SignalAdapter::new();
        let linked = adapter.link("+1234567890".to_string());
        assert_eq!(linked.phone_number(), "+1234567890");
        let verified = linked.verify();
        assert_eq!(verified.phone_number(), "+1234567890");
    }
}

#![deny(unsafe_code)]

//! Signal protocol channel adapter for CrustyClaw.
//!
//! This crate provides the bridge between the CrustyClaw daemon and the Signal
//! messaging protocol. Messages received via Signal are routed into the daemon's
//! message bus, and outbound messages are delivered back through Signal.
//!
//! ## Architecture
//!
//! - **Type-state adapter**: [`SignalAdapter`] enforces `Unlinked → Linked → Verified`
//!   lifecycle at compile time.
//! - **Message types**: [`SignalMessage`], [`Attachment`], [`GroupInfo`] model
//!   the Signal messaging domain.
//! - **Service runner**: [`SignalService`] is the async task that bridges Signal
//!   messages to/from the core daemon's message bus.
//! - **Rate limiter**: [`RateLimiter`] protects against abuse with a token-bucket
//!   algorithm.

pub mod adapter;
pub mod message;
pub mod rate_limit;
pub mod service;

pub use adapter::SignalAdapter;
pub use message::{Attachment, GroupInfo, SignalMessage};
pub use rate_limit::RateLimiter;
pub use service::SignalService;

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

    #[error("rate limited: {0}")]
    RateLimited(String),

    #[error("unsupported media type: {0}")]
    UnsupportedMedia(String),

    #[error("group error: {0}")]
    GroupError(String),
}

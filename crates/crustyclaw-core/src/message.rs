//! Message types for the CrustyClaw message bus.

use std::time::SystemTime;

/// A message envelope routed through the daemon's message bus.
#[derive(Debug, Clone)]
pub struct Envelope {
    /// Unique message identifier.
    pub id: u64,

    /// Timestamp when the message was created.
    pub timestamp: SystemTime,

    /// Source channel (e.g. "signal", "cli", "tui").
    pub channel: String,

    /// Message body.
    pub body: String,

    /// Direction of the message.
    pub direction: Direction,
}

/// Whether a message is inbound (from user) or outbound (to user).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Message received from an external channel.
    Inbound,
    /// Message being sent to an external channel.
    Outbound,
}

impl Envelope {
    /// Create a new inbound message envelope.
    pub fn new(channel: &str, body: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            timestamp: SystemTime::now(),
            channel: channel.to_string(),
            body: body.to_string(),
            direction: Direction::Inbound,
        }
    }

    /// Create an outbound response envelope for this message.
    pub fn reply(&self, body: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);

        Self {
            id: NEXT_ID.fetch_add(1, Ordering::Relaxed),
            timestamp: SystemTime::now(),
            channel: self.channel.clone(),
            body: body.to_string(),
            direction: Direction::Outbound,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_envelope_creation() {
        let envelope = Envelope::new("signal", "Hello");
        assert_eq!(envelope.channel, "signal");
        assert_eq!(envelope.body, "Hello");
        assert_eq!(envelope.direction, Direction::Inbound);
        assert!(envelope.id > 0);
    }

    #[test]
    fn test_envelope_reply() {
        let original = Envelope::new("signal", "Hello");
        let reply = original.reply("World");
        assert_eq!(reply.channel, "signal");
        assert_eq!(reply.body, "World");
        assert_eq!(reply.direction, Direction::Outbound);
        assert_ne!(reply.id, original.id);
    }

    #[test]
    fn test_unique_ids() {
        let a = Envelope::new("a", "x");
        let b = Envelope::new("b", "y");
        assert_ne!(a.id, b.id);
    }
}

//! Token-bucket rate limiter for abuse protection.
//!
//! Limits the number of messages that can be processed per sender within
//! a time window.

use std::collections::HashMap;
use std::time::{Duration, Instant};

/// Configuration for the rate limiter.
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum tokens (messages) per sender.
    pub max_tokens: u32,

    /// How quickly tokens refill (one token per this duration).
    pub refill_interval: Duration,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_tokens: 20,
            refill_interval: Duration::from_secs(3),
        }
    }
}

/// Per-sender token bucket state.
struct Bucket {
    tokens: u32,
    last_refill: Instant,
}

/// A token-bucket rate limiter keyed by sender identity.
pub struct RateLimiter {
    config: RateLimitConfig,
    buckets: HashMap<String, Bucket>,
}

impl RateLimiter {
    /// Create a new rate limiter with the given configuration.
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            buckets: HashMap::new(),
        }
    }

    /// Check whether a message from `sender` should be allowed.
    ///
    /// Returns `true` if the message is allowed (consumes one token),
    /// or `false` if the sender is rate-limited.
    pub fn check(&mut self, sender: &str) -> bool {
        let now = Instant::now();
        let bucket = self.buckets.entry(sender.to_string()).or_insert(Bucket {
            tokens: self.config.max_tokens,
            last_refill: now,
        });

        // Refill tokens based on elapsed time
        let elapsed = now.duration_since(bucket.last_refill);
        let refills = (elapsed.as_millis() / self.config.refill_interval.as_millis()) as u32;
        if refills > 0 {
            bucket.tokens = (bucket.tokens + refills).min(self.config.max_tokens);
            bucket.last_refill = now;
        }

        // Consume a token
        if bucket.tokens > 0 {
            bucket.tokens -= 1;
            true
        } else {
            false
        }
    }

    /// Get the number of remaining tokens for a sender.
    pub fn remaining(&self, sender: &str) -> u32 {
        self.buckets
            .get(sender)
            .map(|b| b.tokens)
            .unwrap_or(self.config.max_tokens)
    }

    /// Remove stale bucket entries that have been fully refilled.
    pub fn cleanup(&mut self) {
        let max = self.config.max_tokens;
        self.buckets.retain(|_, b| b.tokens < max);
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(RateLimitConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rate_limiter_allows_under_limit() {
        let config = RateLimitConfig {
            max_tokens: 5,
            refill_interval: Duration::from_secs(60),
        };
        let mut limiter = RateLimiter::new(config);

        for _ in 0..5 {
            assert!(limiter.check("+1234567890"));
        }
    }

    #[test]
    fn test_rate_limiter_blocks_over_limit() {
        let config = RateLimitConfig {
            max_tokens: 3,
            refill_interval: Duration::from_secs(60),
        };
        let mut limiter = RateLimiter::new(config);

        assert!(limiter.check("+1"));
        assert!(limiter.check("+1"));
        assert!(limiter.check("+1"));
        assert!(!limiter.check("+1")); // 4th should be blocked
    }

    #[test]
    fn test_rate_limiter_independent_senders() {
        let config = RateLimitConfig {
            max_tokens: 2,
            refill_interval: Duration::from_secs(60),
        };
        let mut limiter = RateLimiter::new(config);

        assert!(limiter.check("+1"));
        assert!(limiter.check("+1"));
        assert!(!limiter.check("+1"));

        // Different sender should still have tokens
        assert!(limiter.check("+2"));
        assert!(limiter.check("+2"));
    }

    #[test]
    fn test_remaining_tokens() {
        let config = RateLimitConfig {
            max_tokens: 10,
            refill_interval: Duration::from_secs(60),
        };
        let mut limiter = RateLimiter::new(config);

        // Unknown sender has max tokens
        assert_eq!(limiter.remaining("+1"), 10);

        limiter.check("+1");
        limiter.check("+1");
        assert_eq!(limiter.remaining("+1"), 8);
    }

    #[test]
    fn test_cleanup_removes_full_buckets() {
        let config = RateLimitConfig {
            max_tokens: 5,
            refill_interval: Duration::from_secs(60),
        };
        let mut limiter = RateLimiter::new(config);

        // Create a bucket with consumed tokens
        limiter.check("+1");
        assert_eq!(limiter.buckets.len(), 1);

        // Cleanup should retain (tokens < max)
        limiter.cleanup();
        assert_eq!(limiter.buckets.len(), 1);
    }

    #[test]
    fn test_default_config() {
        let config = RateLimitConfig::default();
        assert_eq!(config.max_tokens, 20);
        assert_eq!(config.refill_interval, Duration::from_secs(3));
    }
}

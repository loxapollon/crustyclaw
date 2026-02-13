#![deny(unsafe_code)]

//! Shared test utilities for the CrustyClaw workspace.
//!
//! Provides reusable fixtures, config builders, and tracing helpers so that
//! individual crate tests stay concise and consistent.
//!
//! Add this crate as a `[dev-dependency]` in any workspace member:
//!
//! ```toml
//! [dev-dependencies]
//! crustyclaw-test-utils = { workspace = true }
//! ```

pub mod config;
pub mod daemon;
pub mod tracing_setup;

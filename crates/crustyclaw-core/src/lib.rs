#![deny(unsafe_code)]

//! CrustyClaw core daemon runtime.
//!
//! Provides the async daemon skeleton including message routing, skill execution,
//! and LLM integration. The daemon is the central process that all other components
//! (CLI, TUI, Signal adapter) communicate with.

pub mod auth;
pub mod build_info;
pub mod daemon;
pub mod logging;
pub mod message;
pub mod plugin;
pub mod security;
pub mod skill;

pub use daemon::Daemon;
pub use logging::{LogCollector, LogReader};
pub use plugin::PluginRegistry;

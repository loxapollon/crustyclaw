#![deny(unsafe_code)]

//! CrustyClaw core daemon runtime.
//!
//! Provides the async daemon skeleton including message routing, skill execution,
//! and LLM integration. The daemon is the central process that all other components
//! (CLI, TUI, Signal adapter) communicate with.

/// Type-state authentication lifecycle (`Unauthenticated → Authenticated → Authorized`).
pub mod auth;
/// Compile-time build metadata (version, git hash, profile).
pub mod build_info;
/// Async daemon runtime and message bus.
pub mod daemon;
/// Apple Virtualization–style sandbox isolation for skills.
pub mod isolation;
/// In-memory log collector for the TUI.
pub mod logging;
/// Message envelope types for the internal bus.
pub mod message;
/// Plugin registry for Forgejo Action extensions.
pub mod plugin;
/// Compile-time security assertions and key management.
pub mod security;
/// Skill trait and runtime registry.
pub mod skill;

pub use daemon::Daemon;
pub use isolation::{Sandbox, SandboxBackend, SandboxConfig};
pub use logging::{LogCollector, LogReader};
pub use plugin::PluginRegistry;

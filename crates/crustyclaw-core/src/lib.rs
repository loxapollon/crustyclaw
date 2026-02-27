#![deny(unsafe_code)]

//! CrustyClaw core daemon runtime.
//!
//! Provides the async daemon skeleton including message routing, skill execution,
//! and LLM integration. The daemon is the central process that all other components
//! (CLI, TUI, Signal adapter) communicate with.

use std::future::Future;
use std::pin::Pin;

/// A type-erased, `Send`-safe, boxed future — the standard return type for async
/// trait methods that require dynamic dispatch (`dyn Trait`).
///
/// Native `async fn` in traits (stable since Rust 1.75) produces opaque return
/// types that are **not** object-safe. Traits consumed via `Box<dyn Trait>` or
/// `&dyn Trait` must return a concrete `Pin<Box<dyn Future>>` instead. This
/// alias keeps those signatures readable.
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Type-state authentication lifecycle (`Unauthenticated → Authenticated → Authorized`).
/// Includes transparent local-identity authentication for CLI/TUI.
pub mod auth;
/// Compile-time build metadata (version, git hash, profile).
pub mod build_info;
/// Context engine — tool registry, codebase indexing, and context window management.
pub mod context;
/// Async daemon runtime and message bus.
pub mod daemon;
/// IPC layer — Unix domain socket transport for CLI/TUI control.
pub mod ipc;
/// Multi-backend sandbox isolation for skills (Docker, Firecracker, Apple VZ, Linux NS, noop).
pub mod isolation;
/// LLM provider integration — multi-provider chat completions with tool use.
pub mod llm;
/// In-memory log collector for the TUI.
pub mod logging;
/// Message envelope types for the internal bus.
pub mod message;
/// Plugin registry for Forgejo Action extensions.
pub mod plugin;
/// Secrets management — loading, storage, zeroization, and container injection.
pub mod secrets;
/// Compile-time security assertions and key management.
pub mod security;
/// Skill trait and runtime registry.
pub mod skill;

pub use auth::LocalIdentity;
pub use daemon::Daemon;
pub use ipc::{IpcClient, IpcState};
pub use isolation::{
    CredentialProxy, DockerSandboxBackend, FirecrackerBackend, IsolationLevel, Sandbox,
    SandboxBackend, SandboxConfig, TrustBasedSelector, TrustTier,
};
pub use logging::{LogCollector, LogReader};
pub use plugin::PluginRegistry;
pub use secrets::SecretStore;

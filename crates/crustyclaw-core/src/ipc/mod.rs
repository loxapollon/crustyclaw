//! Daemon IPC — Unix domain socket transport for CLI/TUI control.
//!
//! The daemon exposes an HTTP/JSON API over a Unix socket. The CLI and TUI
//! connect as clients to query status, request shutdown, evaluate policies,
//! and inspect runtime state.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────┐         Unix socket        ┌──────────────┐
//! │   CLI    │───────────────────────────▶│  IPC Server  │
//! │   TUI    │  HTTP/1.1 + JSON           │  (axum)      │
//! └──────────┘                            └──────┬───────┘
//!                                                │
//!                                         ┌──────▼───────┐
//!                                         │    Daemon    │
//!                                         │   Runtime    │
//!                                         └──────────────┘
//! ```

pub mod client;
pub mod server;
pub mod types;

pub use client::IpcClient;
pub use server::{DEFAULT_SOCKET_PATH, IpcState};
pub use types::*;

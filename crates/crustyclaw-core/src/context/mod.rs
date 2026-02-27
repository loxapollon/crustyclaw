//! Context engine — tool registry, codebase indexing, and context window management.
//!
//! The context engine provides three integrated layers:
//!
//! 1. **Tool Registry** — MCP-compatible tool definitions with per-task scoping
//!    based on trust levels and tags.
//!
//! 2. **Codebase Indexer** — Symbol extraction from source files (functions, structs,
//!    types, etc.) for static context. Uses pattern matching with a future upgrade
//!    path to tree-sitter AST parsing.
//!
//! 3. **Context Window** — Token budget management and priority-based context packing.
//!    Ensures the LLM receives the most relevant context within its token limit.
//!
//! ## Architecture
//!
//! ```text
//! ┌──────────────────────────────────────────────┐
//! │                Context Engine                 │
//! │                                              │
//! │  ┌─────────────┐  ┌──────────┐  ┌─────────┐│
//! │  │ Tool        │  │ Codebase │  │ Context ││
//! │  │ Registry    │  │ Indexer  │  │ Window  ││
//! │  │             │  │          │  │ Manager ││
//! │  │ ·search_code│  │ ·Rust    │  │         ││
//! │  │ ·read_file  │  │ ·TS/JS   │  │ ·Budget ││
//! │  │ ·run_command│  │ ·Python  │  │ ·Pack   ││
//! │  │ ·list_files │  │ ·Go      │  │ ·Assemble│
//! │  └─────────────┘  └──────────┘  └─────────┘│
//! └──────────────────────────────────────────────┘
//! ```

pub mod indexer;
pub mod tools;
pub mod window;

pub use indexer::{Symbol, SymbolIndex, SymbolKind};
pub use tools::{RegisteredTool, ToolRegistry, ToolTrust};
pub use window::{ContextItem, ContextKind, ContextWindow};

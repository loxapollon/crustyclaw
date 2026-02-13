# CrustyClaw

A secure, Rust-based alternative to OpenClaw and NanoClaw, using Forgejo Actions for extension.

## Project Overview

CrustyClaw is a security-first AI agent daemon written in Rust. It replaces OpenClaw
(52+ modules, shared-memory Node.js process) and NanoClaw (500-line TypeScript) with
a single, auditable, memory-safe codebase. Users interact with the agent via Signal.
Operators manage it via a CLI and TUI.

## Architecture

- **Language:** Rust 1.93+ (edition 2024), `#![deny(unsafe_code)]`
- **Core:** Full-async daemon (`tokio`) — message routing, skill execution, LLM integration, OS signal handling (SIGHUP/SIGTERM)
- **Control plane:** CLI (`clap`) for scripting + TUI (`ratatui`) for interactive ops
- **User channel:** Signal (end-to-end encrypted messaging)
- **Extension model:** Forgejo Actions — plugins run as sandboxed CI/CD workflows
- **Security posture:** Memory-safe by default, container isolation for skills,
  no `unsafe` without documented justification, `cargo-audit` supply-chain checks

## Workspace Layout

```
crustyclaw/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── crustyclaw-core/        # daemon runtime, message routing, skill engine
│   ├── crustyclaw-cli/         # CLI control plane (clap)
│   ├── crustyclaw-tui/         # TUI control plane (ratatui + crossterm)
│   ├── crustyclaw-signal/      # Signal protocol channel adapter
│   ├── crustyclaw-macros/      # proc-macro crate (derive, attribute macros)
│   └── crustyclaw-config/      # config loading, validation, policy engine
├── docs/                       # user documentation
├── actions/                    # Forgejo Action extension definitions
├── .forgejo/workflows/         # CI/CD pipelines
└── .claude/plans/              # roadmap and planning docs
```

## Development Guidelines

- All crates must compile with `#![deny(unsafe_code)]` unless explicitly exempted.
- Run `cargo clippy` and `cargo fmt` before every commit.
- Tests are required for all public API surfaces (`cargo test`).
- Prefer compile-time guarantees (const generics, type-state patterns, macros)
  over runtime checks where feasible.
- Forgejo Action extension points should be defined declaratively via derive/attribute macros.
- Sensitive data types must derive `Redact` and `SecureZeroize`.

## Build & Run

```bash
cargo build                    # debug build
cargo build --release          # release build
cargo test --workspace         # run all tests
cargo clippy --workspace       # lint all crates
cargo fmt --all                # format all crates
cargo run -p crustyclaw-cli    # run CLI
cargo run -p crustyclaw-tui    # run TUI
```

## Key Dependencies

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime |
| `clap` | CLI parsing |
| `ratatui` + `crossterm` | TUI framework |
| `serde` + `toml` | Config serialization |
| `presage` or `libsignal` | Signal protocol |
| `syn` + `quote` + `proc-macro2` | Proc-macro infrastructure |
| `zeroize` | Sensitive memory clearing |
| `tracing` | Structured logging |

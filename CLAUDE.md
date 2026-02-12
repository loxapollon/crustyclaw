# CrustyClaw

A secure, Rust-based alternative to OpenClaw and NanoClaw, using Forgejo Actions for extension.

## Project Overview

CrustyClaw is built with security-first principles in Rust, leveraging the language's
ownership model, type system, and zero-cost abstractions to provide a hardened tool
that replaces OpenClaw and NanoClaw with a single, auditable codebase.

## Architecture

- **Language:** Rust (latest stable)
- **Extension model:** Forgejo Actions — plugins and extensions are defined as
  Forgejo Action workflows, keeping the core minimal and the extension surface
  sandboxed.
- **Security posture:** Memory-safe by default, no `unsafe` without documented
  justification and audit, dependency supply-chain checks via `cargo-audit`.

## Development Guidelines

- All code must compile with `#![deny(unsafe_code)]` unless an explicit exemption
  is granted and documented.
- Use `cargo clippy` and `cargo fmt` before every commit.
- Tests are required for all public API surfaces (`cargo test`).
- Prefer compile-time guarantees (const generics, type-state patterns, macros)
  over runtime checks where feasible.
- Forgejo Action extension points should be defined declaratively; prefer derive
  macros or attribute macros for registering new actions.

## Build & Run

```bash
cargo build          # debug build
cargo build --release # release build
cargo test           # run tests
cargo clippy         # lint
cargo fmt            # format
```

## Extension via Forgejo Actions

Extensions are implemented as Forgejo Action workflows that the CrustyClaw core
dispatches to. The core exposes a stable JSON/YAML interface for action definitions.
See the `actions/` directory (when created) for examples.

## Repository Structure

```
├── CLAUDE.md          # this file — project context for Claude
├── README.md          # project readme
├── src/               # Rust source (to be created)
│   └── main.rs
├── actions/           # Forgejo Action extension definitions (to be created)
├── Cargo.toml         # Rust project manifest (to be created)
└── .github/           # CI workflows (to be created)
```

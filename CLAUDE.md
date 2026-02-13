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
├── Cargo.toml                  # workspace root (profiles, deps)
├── .cargo/config.toml          # build flags (frame pointers for profiling)
├── .config/nextest.toml        # nextest test runner configuration
├── crates/
│   ├── crustyclaw-core/        # daemon runtime, message routing, skill engine
│   ├── crustyclaw-cli/         # CLI control plane (clap)
│   ├── crustyclaw-tui/         # TUI control plane (ratatui + crossterm)
│   ├── crustyclaw-signal/      # Signal protocol channel adapter
│   ├── crustyclaw-macros/      # proc-macro crate (derive, attribute macros)
│   ├── crustyclaw-config/      # config loading, validation, policy engine
│   └── crustyclaw-test-utils/  # shared test fixtures, builders, tracing helpers
├── docs/                       # user documentation
├── fuzz/                       # fuzz targets (libfuzzer)
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
cargo build --release          # optimised release (thin LTO, strip symbols)
cargo build --profile profiling  # release + debug info for flamegraphs
cargo test --workspace         # run all tests (built-in harness)
cargo nextest run --workspace  # run tests via nextest (parallel, better output)
cargo clippy --workspace       # lint all crates
cargo fmt --all                # format all crates
cargo run -p crustyclaw-cli    # run CLI
cargo run -p crustyclaw-tui    # run TUI
```

## Testing

### Structure

- **Unit tests**: `#[cfg(test)] mod tests` inline in each source file
- **Integration tests**: `crates/*/tests/*.rs` (separate compilation units)
- **Shared fixtures**: `crustyclaw-test-utils` crate — config builders, daemon helpers, tracing init
- **Fuzz targets**: `fuzz/` directory (config parser, policy evaluator)

### Running

```bash
cargo test --workspace                  # all tests + doctests
cargo nextest run --workspace           # parallel test runner (preferred)
cargo nextest run --workspace --profile ci  # CI mode (retries, JUnit XML)
cargo test --doc --workspace            # doctests only (nextest limitation)
RUST_LOG=debug cargo test               # with tracing output
```

### Key conventions

- Use `tempfile::TempDir` for any test that writes to disk — guarantees cleanup on panic
- Use `pretty_assertions` for struct/string comparisons in tests
- Use `test-log` for automatic tracing subscriber init (`#[test_log::test]`)
- Use `#[tokio::test]` for async tests (single-threaded by default)
- Shared helpers go in `crustyclaw-test-utils`, not duplicated per crate

## Profiling

```bash
# Install flamegraph tooling
cargo install flamegraph

# Build with profiling profile (release + debug symbols, no LTO)
cargo build --profile profiling --bin crustyclaw

# Generate flamegraph
cargo flamegraph --profile profiling --bin crustyclaw

# Linux perf integration
perf record --call-graph=dwarf ./target/profiling/crustyclaw
perf script | inferno-collapse-perf | inferno-flamegraph > flamegraph.svg
```

### Build Profiles

| Profile | Use case | Key settings |
|---------|----------|-------------|
| `dev` | Fast iteration | opt=0, debug=full, heavy deps at opt=2 |
| `release` | Production | opt=3, thin LTO, codegen-units=1, strip, panic=abort |
| `profiling` | Flamegraphs / perf | release + debug symbols, no LTO, no strip |
| `bench` | Benchmarks | release + debug symbols |

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
| `tracing` + `tracing-subscriber` | Structured logging |
| `tracing-flame` | Flamegraph generation from tracing spans |
| `test-log` | Automatic tracing in tests |
| `pretty_assertions` | Readable test diffs |
| `tempfile` | Auto-cleanup temp dirs in tests |

# CrustyClaw

A secure, Rust-based AI agent daemon. Routes messages between Signal and an
LLM-powered skill engine, managed via CLI or interactive TUI. Extensions run as
sandboxed Forgejo Actions.

CrustyClaw replaces OpenClaw (52+ modules, shared-memory Node.js) and NanoClaw
(500-line TypeScript) with a single, auditable, memory-safe codebase.

## Quick start

```bash
# Build
cargo build --workspace

# Run the CLI
cargo run -p crustyclaw-cli -- --help

# Run the TUI
cargo run -p crustyclaw-tui

# Run all tests
cargo test --workspace
```

## Architecture

```
 Operator ──► CLI / TUI ──► Daemon (tokio) ──► Skill engine
                                  │                  │
                                  ├── Signal adapter  ├── Forgejo Actions (sandboxed)
                                  └── Message bus     └── LLM provider
```

| Crate | Purpose |
|-------|---------|
| `crustyclaw-core` | Async daemon runtime, message bus, skill engine, isolation backends |
| `crustyclaw-cli` | CLI control plane (`clap`) — start, stop, config, policy, isolation |
| `crustyclaw-tui` | Interactive TUI (`ratatui` + `crossterm`) — Dashboard, Logs, Messages, Config |
| `crustyclaw-signal` | Signal protocol channel adapter with type-state lifecycle |
| `crustyclaw-macros` | Proc macros: `Redact`, `Validate`, `SecureZeroize`, `ActionPlugin`, `action_hook`, `security_policy!` |
| `crustyclaw-config` | TOML config loading (async I/O), validation, RBAC policy engine |

## Configuration

CrustyClaw reads `crustyclaw.toml` from the working directory. All sections are
optional — sensible defaults are provided.

```toml
[daemon]
listen_addr = "127.0.0.1"
listen_port = 9100

[signal]
enabled = false
data_dir = "data/signal"

[logging]
level = "info"

[isolation]
backend = "auto"            # "auto", "apple-vz", "linux-ns", "noop"
default_memory_bytes = 268435456  # 256 MiB
default_cpu_fraction = 0.5
default_timeout_secs = 60
default_network = "none"    # "none", "host-only", "outbound-only"
max_concurrent = 4

[policy]
default_effect = "deny"

[[policy.rules]]
role = "admin"
action = "*"
resource = "*"
effect = "allow"
priority = 10
```

## OS signal handling

| Signal | Behaviour |
|--------|-----------|
| `SIGHUP` | Reload config from disk. Published via a `watch` channel — running skills are **never** interrupted. Consumers pick up the new config at their next natural pause point. |
| `SIGTERM` | Graceful shutdown — finish in-flight work, then exit. |
| `SIGINT` (Ctrl-C) | Same as SIGTERM. |

## Security

- `#![deny(unsafe_code)]` across all crates
- `#[derive(Redact)]` — redacts sensitive fields in Debug output
- `#[derive(SecureZeroize)]` — zeroizes memory on Drop
- `#[derive(Validate)]` — compile-time validation rules
- RBAC policy engine with priority-ordered rules
- Token-bucket rate limiting per sender
- Sandboxed skill execution (Apple VZ / Linux namespaces / noop)
- `cargo-audit` and `cargo-deny` in CI

See [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md) for the full threat model.

## TUI keybindings

| Key | Action |
|-----|--------|
| `q` | Quit |
| `Tab` / `l` | Next panel |
| `BackTab` / `h` | Previous panel |
| `j` / `k` | Scroll down / up |
| `d` / `u` | Half-page down / up |
| `gg` | Scroll to top |
| `G` | Scroll to bottom |
| `1`-`4` | Jump to panel |

## Development

```bash
cargo clippy --workspace       # lint
cargo fmt --all                # format
cargo test --workspace         # test
cargo doc --workspace --no-deps  # generate docs
```

**Minimum supported Rust version:** 1.93.0 (edition 2024)

## License

MIT

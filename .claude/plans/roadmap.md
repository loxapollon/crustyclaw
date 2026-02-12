# CrustyClaw Roadmap: Research & Implementation

## Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Language** | Rust (stable) | Memory safety, zero-cost abstractions, `#![deny(unsafe_code)]` |
| **Control plane** | CLI + TUI | CLI (`clap`) for scripting/automation, TUI (`ratatui`) for interactive ops |
| **User channel** | Signal | End-to-end encrypted, aligns with security-first positioning |
| **Extension model** | Forgejo Actions | Self-hostable CI/CD plugin system, sandboxed execution |
| **Core runtime** | Async daemon (`tokio`) | Long-running process, message routing, skill execution |

## Crate Layout

```
crustyclaw/
├── Cargo.toml                  # workspace root
├── crates/
│   ├── crustyclaw-core/        # daemon runtime, message routing, skill engine
│   ├── crustyclaw-cli/         # CLI control plane (clap)
│   ├── crustyclaw-tui/         # TUI control plane (ratatui)
│   ├── crustyclaw-signal/      # Signal protocol channel adapter
│   ├── crustyclaw-macros/      # proc-macro crate (derive, attribute macros)
│   └── crustyclaw-config/      # config loading, validation, policy engine
├── actions/                    # Forgejo Action extension definitions
├── .forgejo/workflows/         # CI/CD pipelines
└── .claude/plans/              # this roadmap
```

---

## Phase 0 — Foundation (Complete)
- [x] Initialize repository with README.md and CLAUDE.md
- [x] Research Rust metaprogramming landscape
- [x] Decide on architecture: CLI + TUI control, Signal channel, Forgejo extensions
- [x] Set up Cargo workspace structure

## Phase 1 — Core Scaffolding (Complete)
- [x] Create `Cargo.toml` workspace with crate layout above
- [x] `crustyclaw-core`: async daemon skeleton (`tokio`, signal handling, graceful shutdown)
- [x] `crustyclaw-cli`: `clap`-based CLI with subcommands (`start`, `stop`, `status`, `config`)
- [x] `crustyclaw-macros`: proc-macro crate with `Redact` derive macro
- [x] `crustyclaw-config`: TOML config loading with `serde` + validation
- [x] `crustyclaw-signal`: Signal adapter with type-state lifecycle pattern
- [x] `crustyclaw-tui`: `ratatui` + `crossterm` skeleton with panel navigation
- [x] Set up CI via Forgejo Actions (`.forgejo/workflows/ci.yml`)
- [x] Add `cargo-audit`, `clippy`, `fmt` checks to CI
- [x] Add `#![deny(unsafe_code)]` across all crates

## Phase 2 — TUI Control Plane (Complete)
- [x] `crustyclaw-tui`: `ratatui` + `crossterm` multi-module TUI
- [x] Dashboard panel: daemon status, uptime, connected channels
- [x] Logs panel: live log streaming via `LogCollector` tracing layer
- [x] Config panel: syntax-highlighted TOML display with scrolling
- [x] Message panel: scrollable message list (ready for Signal integration)
- [x] Keybinding system (vim-style: j/k, d/u, gg/G, h/l, Tab, 1-4)

## Phase 3 — Signal Channel Adapter (Complete)
- [x] `crustyclaw-signal`: Multi-module crate (adapter, message, rate_limit, service)
- [x] Type-state pattern: `Unlinked → Linked → Verified` for Signal session lifecycle
- [x] Message types: `SignalMessage`, `Attachment`, `GroupInfo`
- [x] Group chat support (GroupInfo with members)
- [x] Media handling types (images, audio/voice notes, video, files)
- [x] Rate limiting and abuse protection (token-bucket per sender)
- [x] `SignalService` async runner with inbound/outbound message routing
- [ ] Signal protocol integration (`presage` / `signal-cli` binding) — deferred

## Phase 4 — Security Primitives (Complete)
- [x] `#[derive(Validate)]` — field-level validation (non_empty, range, min/max_len)
- [x] `#[derive(Redact)]` — auto-redact sensitive fields in Debug output
- [x] `#[derive(SecureZeroize)]` — zeroize sensitive memory on Drop
- [x] Type-state pattern for auth lifecycle (Unauth → Auth → Authorized)
- [x] `const` assertions for security invariants (key lengths, TLS versions)
- [x] `KeyBuffer<N>` — const-generic sized key buffer with compile-time enforcement
- [x] Build script: embed git commit hash, build timestamp, build profile
- [ ] Container isolation: sandboxed skill execution (seccomp/landlock) — deferred

## Phase 5 — Configuration & Policy Engine
- [ ] Config format: TOML with `serde` + `Validate` derive stacking
- [ ] `security_policy!{}` function-like proc macro for policy DSL
- [ ] Compile-time policy validation (role/action/resource well-formedness)
- [ ] Runtime policy evaluation with zero-cost compiled match trees

## Phase 6 — Forgejo Actions Extension System
- [ ] Define `ActionPlugin` trait
- [ ] `#[derive(ActionPlugin)]` — input parsing, output setters, metadata generation
- [ ] `#[action_hook(event, priority)]` attribute macro for hook registration
- [ ] Build-script: `action.yml` → typed Rust bindings, schema validation
- [ ] Plugin discovery via `inventory`/`linkme`
- [ ] `workflow_step!{}` macro for compile-time workflow fragment validation
- [ ] Integration test harness via `action_integration_test!` declarative macro

## Phase 7 — Hardening & Supply Chain
- [ ] `cargo-vet` or `cargo-crev` integration
- [ ] Dependency pinning and reproducible builds
- [ ] Fuzz testing harness (`cargo-fuzz` / `afl`)
- [ ] SBOM generation in CI
- [ ] Threat model documentation

## Phase 8 — Documentation & Release
- [ ] Crate-level and public-API docs (`cargo doc`)
- [ ] CLI + TUI user documentation
- [ ] Extension authoring guide (Forgejo Action plugins)
- [ ] Signal setup guide
- [ ] Versioned releases via Forgejo Actions

---

## Metaprogramming Strategy Summary

| Technique | Where Used | Priority |
|-----------|-----------|----------|
| `macro_rules!` | Test harnesses, CLI boilerplate, TUI widget patterns | Medium |
| Derive macros | Validate, Redact, SecureZeroize, ActionPlugin | **High** |
| Attribute macros | `#[action_hook]`, extension registration | **High** |
| Function-like proc macros | `security_policy!{}`, `workflow_step!{}` DSLs | Medium |
| `const fn` + const generics | Security invariant assertions, sized buffers | Medium |
| Type-state patterns | Signal session, auth lifecycle, build pipeline states | **High** |
| Build scripts | Git metadata, action.yml codegen, lockfile checks | Medium |

## Key Crate Dependencies (Planned)

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime for daemon |
| `clap` | CLI argument parsing |
| `ratatui` + `crossterm` | TUI framework |
| `serde` + `toml` | Config serialization |
| `presage` or `libsignal` | Signal protocol |
| `syn` + `quote` + `proc-macro2` | Procedural macro infrastructure |
| `zeroize` | Sensitive memory clearing |
| `tracing` | Structured logging |
| `inventory` or `linkme` | Plugin registration |

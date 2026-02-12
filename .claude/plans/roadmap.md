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

## Phase 0 — Foundation (Current)
- [x] Initialize repository with README.md and CLAUDE.md
- [x] Research Rust metaprogramming landscape
- [x] Decide on architecture: CLI + TUI control, Signal channel, Forgejo extensions
- [ ] Set up Cargo workspace structure

## Phase 1 — Core Scaffolding
- [ ] Create `Cargo.toml` workspace with crate layout above
- [ ] `crustyclaw-core`: async daemon skeleton (`tokio`, signal handling, graceful shutdown)
- [ ] `crustyclaw-cli`: `clap`-based CLI with subcommands (`start`, `stop`, `status`, `config`)
- [ ] `crustyclaw-macros`: empty proc-macro crate, wired into workspace
- [ ] `crustyclaw-config`: TOML config loading with `serde` + validation
- [ ] Set up CI via Forgejo Actions (`.forgejo/workflows/ci.yml`)
- [ ] Add `cargo-audit`, `clippy`, `fmt` checks to CI
- [ ] Add `#![deny(unsafe_code)]` across all crates

## Phase 2 — TUI Control Plane
- [ ] `crustyclaw-tui`: `ratatui` + `crossterm` skeleton
- [ ] Dashboard panel: daemon status, uptime, connected channels
- [ ] Logs panel: live log streaming from daemon (via Unix socket or IPC)
- [ ] Config panel: view/edit configuration interactively
- [ ] Message panel: live view of incoming/outgoing Signal messages
- [ ] Keybinding system (vim-style navigation)

## Phase 3 — Signal Channel Adapter
- [ ] Research Signal protocol integration options (`presage`, `signal-cli`, `libsignal`)
- [ ] `crustyclaw-signal`: Signal account linking / registration
- [ ] Message receive pipeline: Signal → core daemon → LLM → response → Signal
- [ ] Group chat support
- [ ] Media handling (images, files, voice notes)
- [ ] Rate limiting and abuse protection
- [ ] Type-state pattern: `Unlinked → Linked → Verified` for Signal session lifecycle

## Phase 4 — Security Primitives (Metaprogramming-Heavy)
- [ ] `#[derive(Validate)]` — compile-time input validation from struct annotations
- [ ] `#[derive(Redact)]` — auto-redact sensitive fields in Debug/Display/logs
- [ ] `#[derive(SecureZeroize)]` — zeroize sensitive memory on Drop
- [ ] Type-state pattern for auth lifecycle (Unauth → Auth → Authorized)
- [ ] `const` assertions for security invariants (key lengths, TLS versions)
- [ ] Build script: embed git commit hash, build timestamp, lockfile checksum
- [ ] Container isolation: sandboxed skill execution (seccomp/landlock on Linux)

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

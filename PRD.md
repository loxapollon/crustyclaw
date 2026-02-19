# CrustyClaw Product Requirements Document

> **Status:** Draft
> **Version:** 0.1.0
> **Date:** 2026-02-19
> **Authors:** CrustyClaw Core Team
> **Related Documents:** [CLAUDE.md](./CLAUDE.md), [Roadmap](./\.claude/plans/roadmap.md), [Threat Model](./docs/THREAT_MODEL.md)

---

## Table of Contents

1. [Vision and Problem Statement](#1-vision-and-problem-statement)
2. [Architecture Overview](#2-architecture-overview)
3. [Core Daemon Runtime](#3-core-daemon-runtime)
4. [CLI Control Plane](#4-cli-control-plane)
5. [TUI Control Plane](#5-tui-control-plane)
6. [Signal Channel Adapter](#6-signal-channel-adapter)
7. [Configuration and Policy Engine](#7-configuration-and-policy-engine)
8. [Security Primitives](#8-security-primitives)
9. [Forgejo Actions Extension System](#9-forgejo-actions-extension-system)
10. [Proc-Macro Infrastructure](#10-proc-macro-infrastructure)
11. [Hardening and Supply Chain](#11-hardening-and-supply-chain)
12. [Non-Goals](#12-non-goals)
13. [Definition of Done](#13-definition-of-done)

---

## 1. Vision and Problem Statement

### 1.1 One-Liner

CrustyClaw is a memory-safe, security-first AI agent daemon that replaces fragile Node.js/TypeScript agent runtimes with a single, auditable Rust codebase — operated via CLI and TUI, extended via Forgejo Actions, and accessed by end users through Signal.

### 1.2 Problem

Existing open-source AI agent platforms present a familiar set of problems:

| System | Architecture | Weakness |
|--------|-------------|----------|
| **OpenClaw** | 52+ Node.js modules, shared-memory monolith | Unbounded attack surface, memory-unsafe, no process isolation between skills, difficult to audit |
| **NanoClaw** | ~500-line TypeScript single-file | No extension model, no access control, no structured ops interface, impossible to harden |

Both share structural deficiencies:

- **Memory safety**: JavaScript/TypeScript offer no compile-time memory guarantees. Use-after-free equivalents (dangling closures, prototype pollution) are endemic.
- **Operator interface**: Neither provides a proper control plane. Configuration is ad-hoc, status inspection requires reading logs, and policy enforcement is absent.
- **Extension model**: OpenClaw's module system runs extensions in-process with full daemon access. NanoClaw has no extension model at all.
- **User channel security**: Neither enforces end-to-end encryption for user interactions. HTTP webhooks and Discord bots transmit messages in cleartext or with server-side encryption only.
- **Auditability**: Shared-memory architectures make it difficult to reason about what code can access what data. Supply chain risks from npm's deep dependency trees compound the problem.

### 1.3 Rationale

Rust is the correct language for this problem because it provides:

1. **Compile-time memory safety** — ownership, borrowing, and lifetime rules eliminate entire vulnerability classes (buffer overflows, use-after-free, data races) without runtime overhead.
2. **`#![deny(unsafe_code)]`** — a hard guarantee, enforced by the compiler, that no crate contains unsafe blocks unless explicitly exempted and documented.
3. **Type-state patterns** — Rust's type system can encode protocol state machines (e.g., `Unlinked → Linked → Verified` for Signal) such that invalid states are unrepresentable at compile time.
4. **Const generics and `const fn`** — security invariants (minimum key sizes, TLS version floors) can be checked at compile time, not runtime.
5. **Auditable dependency tree** — Cargo's lockfile, `cargo-audit`, `cargo-deny`, and `cargo-vet` provide supply chain verification that npm cannot match.

### 1.4 Design Principles

| Principle | Meaning |
|-----------|---------|
| **Memory-safe by default** | `#![deny(unsafe_code)]` in every crate. No unsafe without documented justification and audit trail. |
| **Compile-time over runtime** | Prefer type-state patterns, const generics, const assertions, and proc macros over runtime checks. If a constraint can be caught by the compiler, it must be. |
| **Defense in depth** | Every layer assumes the layers above it are compromised. Skills run in sandboxes. The daemon validates all input. Policy rules default to deny. |
| **Operator-first UX** | The daemon is not user-facing. Operators (the humans running the system) get first-class CLI and TUI interfaces. Users interact only via Signal. |
| **Extension via isolation** | Plugins run as Forgejo Actions — sandboxed CI/CD workflows with no direct access to daemon memory or state. The daemon communicates with extensions through well-defined artifact boundaries. |
| **Minimal surface** | Each crate has a focused responsibility. No crate depends on more than it needs. Public API surfaces are kept small and tested. |

### 1.5 Positioning

```
┌─────────────────────────────────────────────────────────┐
│                    CrustyClaw                            │
│                                                          │
│  What OpenClaw does     →  but memory-safe               │
│  What NanoClaw does     →  but extensible and hardened   │
│  What neither does      →  operator control plane,       │
│                            E2E-encrypted user channel,   │
│                            sandboxed extensions,          │
│                            compile-time security proofs   │
└─────────────────────────────────────────────────────────┘
```

---

## 2. Architecture Overview

### 2.1 System Shape

```
                    ┌───────────────────────────┐
                    │     Operator Zone          │
                    │  ┌─────────┐ ┌─────────┐  │
                    │  │   CLI   │ │   TUI   │  │
                    │  │ (clap)  │ │(ratatui)│  │
                    │  └────┬────┘ └────┬────┘  │
                    │       └─────┬─────┘       │
                    └─────────────┼─────────────┘
                                  │
                    ══════════════╪══════════════  ← trust boundary
                                  ▼
                    ┌──────────────────────────┐
                    │      Core Daemon         │
                    │  ┌──────────────────┐    │
                    │  │   Message Bus    │    │
                    │  │  (broadcast)     │    │
                    │  └──┬─────┬─────┬──┘    │
                    │     │     │     │        │
                    │  ┌──┴──┐┌─┴──┐┌─┴────┐  │
                    │  │Skill││Auth ││Policy│  │
                    │  │ Reg ││ FSM ││Engine│  │
                    │  └─────┘└────┘└──────┘  │
                    └────┬──────┬──────┬──────┘
                         │      │      │
           ══════════════╪══════╪══════╪══════  ← trust boundary
                         │      │      │
                ┌────────┘      │      └────────┐
                ▼               ▼               ▼
        ┌──────────────┐┌──────────────┐┌──────────────┐
        │   Signal     ││    LLM       ││   Forgejo    │
        │   Channel    ││   Provider   ││   Actions    │
        │  (E2E enc)   ││  (external)  ││ (sandboxed)  │
        └──────────────┘└──────────────┘└──────────────┘
```

### 2.2 Workspace Topology

The system is organized as a Cargo workspace with six member crates. Each crate has a single responsibility and minimal dependency surface.

```
crustyclaw/
├── Cargo.toml                  # workspace root (resolver 2, edition 2021)
├── Cargo.lock                  # committed for reproducible builds
├── crates/
│   ├── crustyclaw-core/        # daemon runtime, message bus, skill engine,
│   │                           # auth FSM, isolation, plugin registry
│   ├── crustyclaw-cli/         # CLI control plane (clap 4.x)
│   ├── crustyclaw-tui/         # TUI control plane (ratatui 0.29 + crossterm 0.28)
│   ├── crustyclaw-signal/      # Signal protocol channel adapter
│   ├── crustyclaw-macros/      # proc-macro crate (4 derive + 1 attribute + 1 fn-like)
│   └── crustyclaw-config/      # config loading, validation, RBAC policy engine
├── fuzz/                       # cargo-fuzz targets
├── actions/                    # Forgejo Action extension definitions
├── docs/                       # THREAT_MODEL.md, future guides
└── .forgejo/workflows/         # ci.yml, release.yml
```

### 2.3 Crate Dependency Graph

```
crustyclaw-macros  ←  crustyclaw-config  ←  crustyclaw-core
                                                   ↑
                                          ┌────────┼────────┐
                                          │        │        │
                                    crustyclaw  crustyclaw  crustyclaw
                                       -cli       -tui      -signal
```

- `crustyclaw-macros` is a proc-macro crate with no internal dependencies.
- `crustyclaw-config` depends on `crustyclaw-macros` for derive macros.
- `crustyclaw-core` depends on `crustyclaw-config` for policy evaluation.
- Leaf crates (`cli`, `tui`, `signal`) depend on `crustyclaw-core` and/or `crustyclaw-config`.

### 2.4 Key Dependencies

| Crate | Version | Purpose |
|-------|---------|---------|
| `tokio` | 1.x (full) | Async runtime for daemon and all I/O |
| `clap` | 4.x (derive) | CLI argument parsing and help generation |
| `ratatui` | 0.29 | Terminal UI framework |
| `crossterm` | 0.28 | Cross-platform terminal backend |
| `serde` | 1.x (derive) | Serialization / deserialization framework |
| `toml` | 0.8 | TOML config file parsing |
| `syn` | 2.x (full) | Proc-macro token parsing |
| `quote` | 1.x | Proc-macro code generation |
| `proc-macro2` | 1.x | Proc-macro token stream abstraction |
| `zeroize` | 1.x (derive) | Secure memory clearing |
| `tracing` | 0.1 | Structured logging facade |
| `tracing-subscriber` | 0.3 (env-filter) | Log subscriber with runtime filter |
| `thiserror` | 2.x | Typed error derivation |
| `anyhow` | 1.x | Contextual error propagation |

---

## 3. Core Daemon Runtime

### 3.1 Overview

The core daemon (`crustyclaw-core`) is the central process. It owns the async runtime, message bus, skill registry, authentication state machine, sandbox backend, and plugin registry. All other crates communicate with the daemon through its public API.

### 3.2 Message Bus

The daemon uses `tokio::sync::broadcast` with a bounded capacity of 256 entries.

```rust
pub struct Daemon {
    bus: broadcast::Sender<Envelope>,
    shutdown: CancellationToken,
    skills: SkillRegistry,
    plugins: PluginRegistry,
}
```

**Envelope model:**
```rust
pub struct Envelope {
    pub direction: Direction,  // Inbound | Outbound
    pub payload: String,
    pub timestamp: u64,
}
```

All inbound messages (from Signal, CLI commands, or extensions) and outbound messages (to Signal users, logs, or extension artifacts) flow through the bus as `Envelope` values. Subscribers receive all messages; filtering is the subscriber's responsibility.

### 3.3 Daemon Lifecycle

1. **Startup** — Load config, build policy engine, initialize skill registry, bind message bus, register plugins.
2. **Running** — Async select loop: process bus messages, execute skills, handle shutdown signals.
3. **Shutdown** — `CancellationToken` propagation, graceful drain of in-flight messages, `Drop` cleanup.

The daemon handles `SIGINT` and `SIGTERM` via `tokio::signal` for graceful shutdown.

### 3.4 Skill Engine

Skills are the daemon's unit of work. Each skill implements the `Skill` trait:

```rust
#[async_trait]
pub trait Skill: Send + Sync {
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    async fn execute(&self, input: &str) -> Result<String>;
}
```

The `SkillRegistry` provides runtime lookup by name. `IsolatedSkill` wraps any `Skill` with a `SandboxConfig` for execution inside a container backend.

### 3.5 Authentication State Machine

Type-state pattern ensures invalid auth transitions are compile-time errors:

```
Session<Unauthenticated>
    → .authenticate(credentials)
        → Session<Authenticated>
            → .authorize(policy_engine)
                → Session<Authorized>
```

Only `Session<Authorized>` exposes the `.execute_action()` method. Attempting to call it on `Session<Unauthenticated>` or `Session<Authenticated>` is a type error, not a runtime error.

### 3.6 Sandbox Isolation

Skills that require isolation execute inside a sandbox backend. The design follows a trait-based abstraction:

```rust
pub trait SandboxBackend: Send + Sync {
    fn name(&self) -> &str;
    fn available(&self) -> bool;
    fn execute(&self, config: &SandboxConfig, command: &str)
        -> Result<String>;
}
```

**Backend implementations:**

| Backend | Platform | Isolation Level |
|---------|----------|----------------|
| `AppleVzBackend` | macOS | Apple Virtualization.framework |
| `LinuxNamespaceBackend` | Linux | cgroups + namespaces |
| `NoopBackend` | Any | No isolation (testing only) |

**SandboxConfig** provides declarative resource limits:
- CPU cores and fraction
- Memory ceiling
- Execution timeout
- Max open files and processes
- Filesystem mounts (read-only, read-write)
- Network policy (`none`, `host-only`, `outbound-only`, allow-list)

### 3.7 Log Collector

The `LogCollector` is a `tracing` layer that captures log entries into a bounded ring buffer (1000 entries). The `LogReader` provides a read handle for the TUI's live log panel. This architecture decouples log production from consumption without filesystem I/O.

### 3.8 Build Metadata

A `build.rs` script embeds compile-time metadata:
- `VERSION` — from `Cargo.toml`
- `GIT_HASH` — short hash via `git rev-parse --short HEAD`
- `BUILD_TIMESTAMP` — Unix epoch seconds
- `BUILD_PROFILE` — `debug` or `release`

Exposed via the `build_info` module for the CLI `version` subcommand and TUI dashboard.

---

## 4. CLI Control Plane

### 4.1 Overview

The CLI (`crustyclaw-cli`) is the scripting and automation interface for operators. Built with `clap` 4.x using derive macros.

### 4.2 Command Structure

```
crustyclaw [OPTIONS] <COMMAND>

Options:
  -c, --config <FILE>    Config file path [default: crustyclaw.toml]
  -v, --verbose          Increase verbosity (-v, -vv, -vvv)

Commands:
  start       Launch the daemon with loaded configuration
  stop        Stop a running daemon instance
  status      Query daemon health and runtime status
  config      Validate and optionally display configuration
    --show    Print resolved config to stdout
  version     Display build metadata (version, git hash, profile)
  policy      Evaluate a policy check
    --role    Role to evaluate
    --action  Action to evaluate
    --resource Resource to evaluate
  plugins     List registered plugins and hooks
  isolation   Show isolation backend availability and status
```

### 4.3 Design Decisions

- **Verbosity levels** map directly to `tracing` filter levels: `-v` = info, `-vv` = debug, `-vvv` = trace.
- **Config validation** runs on every command invocation. Invalid config fails fast before any daemon operation.
- **Policy evaluation** is a dry-run diagnostic: operators can test `role × action × resource` tuples against the loaded policy without starting the daemon.

---

## 5. TUI Control Plane

### 5.1 Overview

The TUI (`crustyclaw-tui`) provides an interactive, real-time operator interface built with `ratatui` 0.29 and `crossterm` 0.28.

### 5.2 Panel Layout

The TUI implements a four-panel tabbed interface:

| Panel | Key | Content |
|-------|-----|---------|
| **Dashboard** | `1` | Daemon status, uptime, listen address, Signal state, log level |
| **Logs** | `2` | Live-streaming log entries from `LogCollector` ring buffer |
| **Messages** | `3` | Scrollable list of Signal messages (inbound and outbound) |
| **Config** | `4` | Syntax-highlighted TOML configuration display |

### 5.3 Keybinding System

Vim-style keybindings with modal navigation:

| Key | Action |
|-----|--------|
| `j` / `k` | Scroll down / up |
| `d` / `u` | Half-page down / up |
| `g` `g` / `G` | Jump to top / bottom |
| `h` / `l` | Previous / next panel |
| `Tab` | Cycle panel forward |
| `1`–`4` | Direct panel selection |
| `q` | Quit |

### 5.4 Log Streaming Architecture

The TUI subscribes to the daemon's `LogCollector` via the `LogReader` handle. The event loop polls for new log entries on each tick (configurable interval) and appends them to the log panel's display buffer. No filesystem I/O is required — logs flow directly from the `tracing` layer through shared memory.

---

## 6. Signal Channel Adapter

### 6.1 Overview

The Signal adapter (`crustyclaw-signal`) provides end-to-end encrypted messaging between the CrustyClaw daemon and end users. Signal is the only user-facing channel — all user interactions flow through Signal's protocol.

### 6.2 Type-State Lifecycle

The adapter enforces a three-state protocol at the type level:

```
SignalAdapter<Unlinked>
    → .link(phone_number)
        → SignalAdapter<Linked>
            → .verify()
                → SignalAdapter<Verified>
```

Only `SignalAdapter<Verified>` exposes `.send()` and `.receive()` methods. Calling `.send()` on `SignalAdapter<Unlinked>` is a compile-time error.

### 6.3 Message Model

```rust
pub struct SignalMessage {
    pub sender: String,
    pub body: String,
    pub timestamp: u64,
    pub group: Option<GroupInfo>,
    pub attachments: Vec<Attachment>,
}

pub struct GroupInfo {
    pub id: String,
    pub name: String,
    pub members: Vec<String>,
}

pub struct Attachment {
    pub filename: String,
    pub content_type: String,
    pub size: usize,
}
```

**Supported media types:** images, audio/voice notes, video, generic files.

### 6.4 Rate Limiting

Token-bucket rate limiter, configurable per sender:

- **Max tokens** — burst capacity per sender
- **Refill interval** — token replenishment rate
- **Behavior** — messages exceeding the rate are rejected with `RateLimited` error

This prevents abuse from individual users without affecting other senders.

### 6.5 Signal Service

`SignalService` is an async task that bridges the Signal adapter to the daemon's message bus:

1. **Inbound path:** Poll Signal → deserialize → wrap in `Envelope` → publish to bus.
2. **Outbound path:** Subscribe to bus → filter outbound envelopes → serialize → send via Signal.

### 6.6 Error Model

| Error | Meaning |
|-------|---------|
| `LinkingFailed` | Phone number linking rejected or timed out |
| `VerificationFailed` | Verification code invalid or expired |
| `SendFailed` | Message delivery to Signal servers failed |
| `ReceiveFailed` | Message retrieval from Signal servers failed |
| `RateLimited` | Sender exceeded token bucket limit |
| `UnsupportedMedia` | Attachment type not in allow-list |
| `GroupError` | Group operation failed (e.g., unknown group ID) |

### 6.7 Deferred Work

Signal protocol integration (`presage` or `libsignal` binding) is architecturally prepared but not yet connected. The adapter skeleton, type-state lifecycle, and message types are complete. Wire protocol integration is deferred to a future phase.

---

## 7. Configuration and Policy Engine

### 7.1 Overview

The config crate (`crustyclaw-config`) handles TOML configuration loading, structural validation, and a role-based access control (RBAC) policy engine.

### 7.2 Configuration Schema

```toml
[daemon]
listen_addr = "127.0.0.1"
listen_port = 8080

[signal]
enabled = true
data_dir = "data/signal"

[logging]
level = "info"

[isolation]
backend = "auto"          # auto | apple-vz | linux-ns | noop
memory_mb = 512
cpu_fraction = 0.5
timeout_secs = 300
network_policy = "none"   # none | host-only | outbound-only
max_concurrent = 4

[policy]
default_effect = "deny"

[[policy.rules]]
role = "admin"
action = "*"
resource = "*"
effect = "allow"
priority = 100

[[policy.rules]]
role = "user"
action = "read"
resource = "messages"
effect = "allow"
priority = 50
```

### 7.3 Validation Rules

Validation runs at config load time. Invalid config halts startup.

| Field | Constraint |
|-------|-----------|
| `daemon.listen_addr` | Non-empty string |
| `daemon.listen_port` | Non-zero `u16` |
| `isolation.backend` | One of: `auto`, `apple-vz`, `linux-ns`, `noop` |
| `isolation.cpu_fraction` | `(0.0, 1.0]` |
| `isolation.memory_mb` | Non-zero |
| `isolation.network_policy` | One of: `none`, `host-only`, `outbound-only` |
| `policy.rules[].effect` | One of: `allow`, `deny` |
| `policy.rules[].role` | Non-empty string |

### 7.4 Policy Engine

The policy engine evaluates `(role, action, resource)` tuples against a priority-ordered rule set.

**Evaluation algorithm:**

1. Sort rules by priority (highest first).
2. For each rule, test `role`, `action`, and `resource` against the request. `"*"` matches any value.
3. Return the effect (`Allowed` or `Denied`) of the first matching rule.
4. If no rule matches, return the `default_effect` from config.

**Result type:**
```rust
pub enum PolicyDecision {
    Allowed,
    Denied,
    NoMatch,
}
```

`NoMatch` is resolved to `Allowed` or `Denied` based on `policy.default_effect`.

### 7.5 Policy DSL Macro

The `security_policy!{}` function-like proc macro provides a compile-time DSL for inline policy definitions:

```rust
security_policy! {
    allow admin * *;
    deny * write secrets [priority=100];
    allow operator read logs;
}
```

This generates a `Vec<PolicyRule>` at compile time with validation (effect must be `allow` or `deny`, roles must be non-empty).

---

## 8. Security Primitives

### 8.1 Overview

CrustyClaw's security model is built on compile-time guarantees wherever possible. Runtime checks exist only at system boundaries (user input, external API responses, config files).

### 8.2 Compile-Time Security Invariants

| Invariant | Mechanism | Enforcement |
|-----------|-----------|-------------|
| No unsafe code | `#![deny(unsafe_code)]` | Compiler error in every crate |
| Minimum key size | `assert_key_size::<N>()` | `const` assertion, fails compilation if `N < 32` (256 bits) |
| TLS version floor | `assert_tls_version::<V>()` | `const` assertion, fails compilation if `V < 12` (TLS 1.2) |
| Auth state machine | Type-state `Session<S>` | Type error to access authorized resources without full auth chain |
| Signal lifecycle | Type-state `SignalAdapter<S>` | Type error to send/receive without completing Unlinked → Linked → Verified |
| Key buffer sizing | `KeyBuffer<N>` | Const-generic sized buffer, compile-time size enforcement |

### 8.3 Sensitive Data Protection

**`#[derive(Redact)]`** — Automatically redacts fields marked with `#[redact]` in `Debug` output:

```rust
#[derive(Redact)]
struct ApiCredentials {
    pub endpoint: String,
    #[redact]
    pub api_key: String,  // Debug output: "[REDACTED]"
}
```

**`#[derive(SecureZeroize)]`** — Generates a `Drop` implementation that overwrites sensitive memory with zeros:

```rust
#[derive(SecureZeroize)]
struct SessionKey {
    key_material: Vec<u8>,   // zeroed on Drop
    #[no_zeroize]
    key_id: String,          // not zeroed
}
```

### 8.4 Input Validation

**`#[derive(Validate)]`** — Field-level validation with declarative attributes:

```rust
#[derive(Validate)]
struct UserInput {
    #[validate(non_empty)]
    username: String,

    #[validate(range(min = 1, max = 65535))]
    port: u16,

    #[validate(min_len = 8, max_len = 128)]
    password: String,
}
```

Generates `fn validate(&self) -> Result<(), Vec<String>>`.

### 8.5 Trust Boundaries

The system enforces three trust boundaries:

1. **Operator → Daemon** — CLI/TUI commands are trusted after policy evaluation. Operators are authenticated via the auth state machine.
2. **Daemon → Signal** — User messages cross from untrusted territory. All user input is validated and rate-limited before processing.
3. **Daemon → Extensions** — Forgejo Actions run in sandboxed containers. No shared memory, no direct daemon access. Communication is via artifacts and environment variables only.

---

## 9. Forgejo Actions Extension System

### 9.1 Overview

CrustyClaw uses Forgejo Actions as its extension model. Plugins are defined as Forgejo Action workflows — they run in isolated CI/CD containers, receive input via environment variables, and produce output via artifacts. This design provides sandbox isolation by default without requiring the daemon to implement its own container runtime for extensions.

### 9.2 Plugin Scaffolding

**`#[derive(ActionPlugin)]`** generates plugin metadata and input parsing:

```rust
#[derive(ActionPlugin)]
#[action(name = "weather-lookup", version = "1.0.0", description = "Look up weather data")]
struct WeatherPlugin {
    #[action_input(required, description = "City name")]
    city: String,

    #[action_input(default = "metric", description = "Unit system")]
    units: String,
}
```

Generates:
- `fn plugin_name() -> &'static str`
- `fn plugin_version() -> &'static str`
- `fn from_env() -> Result<Self>` (reads `INPUT_CITY`, `INPUT_UNITS` from environment)

### 9.3 Hook System

**`#[action_hook(event = "...", priority = N)]`** registers functions as event hooks:

```rust
#[action_hook(event = "message.received", priority = 10)]
fn on_message(msg: &str) -> Result<()> {
    // handle incoming message
}
```

Hooks are stored in the `PluginRegistry` and dispatched by event name in priority order (highest first).

### 9.4 Plugin Registry

The `PluginRegistry` in `crustyclaw-core` provides:
- **Registration** — `register_plugin(name, version, description)`
- **Hook registration** — `register_hook(event, priority, handler)`
- **Lookup** — `get_plugin(name)` returns plugin metadata
- **Hook dispatch** — `get_hooks(event)` returns handlers sorted by priority (descending)

### 9.5 Deferred Work

- Build-script codegen: `action.yml` → typed Rust bindings
- `workflow_step!{}` macro for composing multi-step actions
- Integration test framework (`action_integration_test!` macro)

---

## 10. Proc-Macro Infrastructure

### 10.1 Overview

The macro crate (`crustyclaw-macros`) provides six macros that encode security, validation, and extension concerns at compile time.

### 10.2 Macro Inventory

| Macro | Kind | Purpose |
|-------|------|---------|
| `#[derive(Redact)]` | Derive | Auto-redact `#[redact]` fields in `Debug` output |
| `#[derive(Validate)]` | Derive | Generate `validate()` from field-level attributes |
| `#[derive(SecureZeroize)]` | Derive | Generate `Drop` impl that zeroizes fields (respects `#[no_zeroize]`) |
| `#[derive(ActionPlugin)]` | Derive | Generate plugin metadata and `from_env()` constructor |
| `#[action_hook(...)]` | Attribute | Register function as prioritized event hook |
| `security_policy!{...}` | Function-like | DSL for declaring RBAC policy rules at compile time |

### 10.3 Implementation

All macros use `syn` 2.x for parsing and `quote` 1.x for code generation. The crate is `proc-macro = true` with no internal workspace dependencies, ensuring it can be used by all other crates without circular dependency issues.

---

## 11. Hardening and Supply Chain

### 11.1 CI Pipeline

The CI pipeline (`.forgejo/workflows/ci.yml`) runs four jobs:

| Job | Steps |
|-----|-------|
| **check** | `cargo fmt --check`, `cargo clippy -D warnings`, `cargo build`, `cargo test`, `cargo doc` |
| **audit** | `cargo-audit`, `Cargo.lock` presence verification |
| **msrv** | Build against MSRV (1.75.0) to prevent accidental edition/feature creep |
| **deny** | `cargo-deny` for license compliance and advisory database checks |

### 11.2 Release Pipeline

The release pipeline (`.forgejo/workflows/release.yml`) triggers on version tags:

1. Run full test suite
2. Build release binaries (`crustyclaw`, `crustyclaw-tui`)
3. Package as tarball
4. Generate SBOM (CycloneDX JSON via `cargo-cyclonedx`)
5. Create Forgejo release with binaries, tarball, and SBOM

### 11.3 Fuzz Testing

Two `cargo-fuzz` targets in `fuzz/`:

| Target | What It Tests |
|--------|--------------|
| `fuzz_config_parser` | Feeds arbitrary bytes to `AppConfig::parse()` to find panics, hangs, or memory issues in TOML deserialization |
| `fuzz_policy_eval` | Feeds arbitrary `(role, action, resource)` strings to the policy engine to find panics or incorrect evaluation |

### 11.4 Threat Model

A comprehensive threat model (`docs/THREAT_MODEL.md`) documents:
- Trust boundary diagram
- Asset inventory with sensitivity ratings
- 10 threat scenarios (T1–T10) with mitigations and implementation status
- Compile-time security invariants
- Residual risks

### 11.5 Supply Chain Measures

| Measure | Tool | Status |
|---------|------|--------|
| Dependency vulnerability scanning | `cargo-audit` | CI-enforced |
| License compliance | `cargo-deny` | CI-enforced |
| Reproducible builds | `Cargo.lock` committed | Enforced |
| SBOM generation | `cargo-cyclonedx` | Release pipeline |
| Dependency vetting | `cargo-vet` / `cargo-crev` | Deferred |

---

## 12. Non-Goals

The following are explicitly out of scope for CrustyClaw:

| Non-Goal | Rationale |
|----------|-----------|
| **Web UI** | Operators use CLI and TUI. A web interface increases attack surface without proportional value for the target user base. |
| **Multi-channel support** | Signal is the sole user channel. Adding Discord, Slack, or HTTP webhooks would dilute the E2E encryption guarantee. |
| **In-process plugins** | Extensions run as Forgejo Actions in isolated containers. In-process plugin loading (e.g., dynamic `.so` loading) is explicitly rejected — it violates memory safety guarantees. |
| **Windows support** | The daemon targets Linux and macOS. Windows support would require significant platform-specific code for sandbox backends and Signal integration. |
| **LLM training or fine-tuning** | CrustyClaw is an inference consumer, not a training platform. LLM providers are external services. |
| **User authentication** | Users are identified by their Signal identity. The daemon does not maintain a separate user database or login system. |
| **GUI-based configuration** | Configuration is file-based (TOML). A GUI config editor is out of scope. |
| **Backward compatibility with OpenClaw/NanoClaw** | CrustyClaw is a clean-room replacement, not a migration path. No import/export tools for existing OpenClaw modules or NanoClaw scripts. |

---

## 13. Definition of Done

### 13.1 Core Daemon

- [ ] Daemon starts, processes messages, and shuts down gracefully under `SIGINT`/`SIGTERM`
- [ ] Message bus delivers envelopes to all subscribers with bounded backpressure
- [ ] Skill registry supports runtime registration and lookup by name
- [ ] `IsolatedSkill` delegates execution to the configured sandbox backend
- [ ] Auth state machine prevents unauthorized access at the type level
- [ ] Log collector captures entries without blocking the daemon event loop
- [ ] Build metadata (version, git hash, profile) is accurate in release builds

### 13.2 CLI Control Plane

- [ ] All subcommands (`start`, `stop`, `status`, `config`, `version`, `policy`, `plugins`, `isolation`) parse and execute correctly
- [ ] `--config` flag overrides default config path
- [ ] `-v` / `-vv` / `-vvv` verbosity levels map to correct tracing filters
- [ ] `policy --role R --action A --resource R` returns the correct policy decision
- [ ] Invalid config produces a clear, actionable error message and exits non-zero

### 13.3 TUI Control Plane

- [ ] Four panels render correctly in terminals ≥ 80×24
- [ ] Vim keybindings navigate panels and scroll content
- [ ] Log panel streams entries in real time from `LogCollector`
- [ ] Config panel displays TOML with syntax highlighting
- [ ] Graceful terminal cleanup on exit (no corrupted terminal state)

### 13.4 Signal Channel Adapter

- [ ] Type-state lifecycle prevents send/receive on unverified adapters at compile time
- [ ] Rate limiter correctly throttles per-sender message rates
- [ ] `SignalService` routes inbound messages to the bus and outbound envelopes to Signal
- [ ] All error types are defined and returned for corresponding failure modes
- [ ] Group messages and attachments are correctly modeled

### 13.5 Configuration and Policy Engine

- [ ] Valid TOML config loads without error
- [ ] Invalid config fields produce specific validation errors
- [ ] Policy engine evaluates `(role, action, resource)` tuples with correct priority ordering
- [ ] Wildcard `"*"` matches any value in role, action, and resource positions
- [ ] `default_effect` is applied when no rule matches
- [ ] `security_policy!{}` macro generates correct `PolicyRule` vectors

### 13.6 Security Primitives

- [ ] `#![deny(unsafe_code)]` is present in every crate's `lib.rs` or `main.rs`
- [ ] `#[derive(Redact)]` replaces `#[redact]` field values with `[REDACTED]` in Debug output
- [ ] `#[derive(SecureZeroize)]` zeros field memory on Drop (verified by test)
- [ ] `#[derive(Validate)]` rejects values outside declared constraints
- [ ] `assert_key_size::<N>()` fails compilation for `N < 32`
- [ ] `assert_tls_version::<V>()` fails compilation for `V < 12`

### 13.7 Forgejo Actions Extension System

- [ ] `#[derive(ActionPlugin)]` generates correct metadata accessors and `from_env()`
- [ ] `#[action_hook]` registers hooks with correct event and priority
- [ ] `PluginRegistry` returns hooks in priority-descending order
- [ ] Plugin lookup by name returns correct metadata

### 13.8 Hardening and Supply Chain

- [ ] CI pipeline passes: fmt, clippy, build, test, doc, audit, MSRV, deny
- [ ] Release pipeline produces binaries, tarball, and SBOM
- [ ] Fuzz targets run without panics for ≥ 1 million iterations
- [ ] Threat model documents all trust boundaries, assets, and threats with mitigations
- [ ] `Cargo.lock` is committed and CI verifies its presence

### 13.9 Cross-Cutting Concerns

- [ ] All public API surfaces have unit tests
- [ ] `cargo clippy --workspace` produces zero warnings
- [ ] `cargo fmt --all --check` produces zero diffs
- [ ] `cargo test --workspace` passes with zero failures
- [ ] `cargo doc --workspace --no-deps` produces zero warnings
- [ ] No crate has more than one `#[allow(unsafe_code)]` without documented justification

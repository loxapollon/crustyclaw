# CrustyClaw Roadmap: Research & Implementation

## Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| **Language** | Rust 1.93+ (edition 2024) | Memory safety, zero-cost abstractions, `#![deny(unsafe_code)]` |
| **Control plane** | CLI + TUI | CLI (`clap`) for scripting/automation, TUI (`ratatui`) for interactive ops |
| **User channel** | Signal | End-to-end encrypted, aligns with security-first positioning |
| **Extension model** | Forgejo Actions | Self-hostable CI/CD plugin system, sandboxed execution |
| **Core runtime** | Full-async daemon (`tokio`) | Message routing, skill execution, OS signal handling (SIGHUP/SIGTERM) |
| **Sandbox isolation** | Multi-backend (L1–L3) | Docker Sandbox, Firecracker, Linux NS, Apple VZ — isolation level per trust tier |
| **Context engine** | MCP tool server | Centralised tool registry (inspired by Stripe Toolshed), per-task scoping |
| **Agent orchestration** | Blueprints | Directed graphs: deterministic nodes + LLM agent loops |
| **Review model** | Configurable HITL | Automated gates + optional human-in-the-loop enforcement |

## System Layers

CrustyClaw's agent runtime is organised into six layers, each independently
configurable and auditable. See [docs/research/stripe-minions-docker-sandboxes.md](../../docs/research/stripe-minions-docker-sandboxes.md)
for the industry research motivating this design.

```
┌─────────────────────────────────────────────────────────────┐
│                    6. Feedback Loop                          │
│    Metrics collection, outcome tracking, model improvement   │
│    Optional human-in-the-loop enforcement at any stage       │
├─────────────────────────────────────────────────────────────┤
│                    5. Review                                 │
│    Automated: lint, type-check, test, policy eval            │
│    Human: configurable approval gates, escalation policy     │
├─────────────────────────────────────────────────────────────┤
│                    4. Execution                              │
│    Isolated sandbox runs (Docker Sandbox / Firecracker /     │
│    Linux NS / Apple VZ / Noop), CI integration               │
├─────────────────────────────────────────────────────────────┤
│                    3. Planning                               │
│    Blueprints: deterministic nodes + LLM agent loops         │
│    Task decomposition, CI budget, escalation thresholds      │
├─────────────────────────────────────────────────────────────┤
│                    2. Context Engine                         │
│    MCP tool server (Toolshed-inspired), static codebase      │
│    context, per-task tool scoping, RAG integration            │
├─────────────────────────────────────────────────────────────┤
│                    1. Sandboxing                             │
│    L1 Containers │ L2 gVisor │ L3 MicroVMs │ L4/L5 future   │
│    Trust-based isolation selection, credential proxying       │
└─────────────────────────────────────────────────────────────┘
```

## Crate Layout

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
│   └── research/               # industry research and competitive analysis
├── fuzz/                       # fuzz targets (libfuzzer)
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
- [x] Container isolation: Apple VZ–style sandbox with backend trait, Linux NS, and noop backends

## Phase 5 — Configuration & Policy Engine (Complete)
- [x] Policy engine: role-based access control with priority-ordered rules
- [x] TOML-configurable policy rules (`[[policy.rules]]` table arrays)
- [x] `security_policy!{}` function-like proc macro for policy DSL
- [x] Compile-time policy validation (effect must be allow/deny, role non-empty)
- [x] Runtime policy evaluation with wildcard matching and priority ordering
- [x] `AppConfig::build_policy_engine()` — config → PolicyEngine bridge

## Phase 6 — Forgejo Actions Extension System (Complete)
- [x] `#[derive(ActionPlugin)]` — metadata, input parsing, `from_env()` generation
- [x] `#[action_hook(event, priority)]` attribute macro for hook registration
- [x] `PluginRegistry` in core — runtime plugin/hook discovery and lookup
- [x] Hook priority ordering (highest-first evaluation)
- [ ] Build-script: `action.yml` → typed Rust bindings — deferred
- [ ] `workflow_step!{}` macro — deferred
- [ ] `action_integration_test!` declarative macro — deferred

## Phase 7 — Hardening & Supply Chain (Complete)
- [x] Enhanced CI: MSRV check (1.93.0), `cargo-deny` licenses/advisories, `-D warnings`
- [x] Cargo.lock committed for reproducible builds
- [x] Fuzz testing harness (`cargo-fuzz`): config parser + policy engine targets
- [x] SBOM generation in release workflow (`cargo-cyclonedx`)
- [x] Threat model documentation (`docs/THREAT_MODEL.md`)
- [ ] `cargo-vet` or `cargo-crev` integration — deferred

## Phase 8 — Documentation & Release (Complete)
- [x] Crate-level and public-API docs across all crates
- [x] CLI expansion: `version`, `policy`, `plugins` subcommands with rich output
- [x] Versioned release workflow via Forgejo Actions (`.forgejo/workflows/release.yml`)
- [x] Edition 2024, Rust 1.93, full async I/O, OS signals (SIGHUP/SIGTERM)
- [x] Comprehensive user docs (`docs/`), README, CLAUDE.md
- [x] TUI test coverage (44 tests), workspace test infrastructure
- [x] `crustyclaw-test-utils` crate, nextest config, build profiles
- [x] Industry research: Stripe Minions, Docker Sandboxes (`docs/research/`)
- [ ] Extension authoring guide (Forgejo Action plugins) — deferred
- [ ] Signal setup guide — deferred (pending protocol integration)

---

## Phase 9 — Sandboxing Layer (Planned)

**Goal:** Upgrade isolation from trait stubs to production-ready multi-backend
sandboxing, with trust-based isolation level selection.

*Research: [Stripe Minions & Docker Sandboxes](../../docs/research/stripe-minions-docker-sandboxes.md)*

- [ ] **Docker Sandbox backend** — `DockerSandboxBackend` using `docker sandbox` CLI
  - MicroVM-level isolation (each skill gets its own VM + private Docker daemon)
  - Credential proxy: API keys never exist inside the sandbox
  - Network allow/deny lists per skill
  - Workspace sync at consistent absolute paths
- [ ] **Firecracker backend** — for self-hosted / headless Linux deployments
  - Direct KVM microVM management via Firecracker API
  - ~150 ms boot, <5 MiB overhead per VM
  - Suitable for multi-tenant agent execution
- [ ] **Trust-based isolation selection**
  - Policy engine selects isolation level based on skill trust tier:
    - `trusted` → L1 (container / noop)
    - `internal` → L2 (gVisor / Linux NS)
    - `untrusted` / `llm-generated` → L3 (microVM)
  - Configurable per-skill overrides in `crustyclaw.toml`
- [ ] **Credential proxying** — sentinel-value swapping for all sandbox backends
  - API keys injected at the network proxy layer, never in sandbox memory
  - Audit log of credential access
- [ ] **Resource limits** — memory, CPU, timeout, network per-sandbox
  - Enforcement at the backend level (cgroups / VM limits)
  - Policy-driven defaults with per-skill overrides

## Phase 10 — Context Engine (Planned)

**Goal:** Build a centralised MCP-based context server (inspired by Stripe's
Toolshed) that provides skills with curated, scoped access to tools and codebase
knowledge.

- [ ] **MCP tool server** — `crustyclaw-mcp` crate
  - Expose internal tools (file search, code navigation, config access) via MCP
  - Third-party SaaS tool adapters (git forge, CI, issue tracker)
  - Tool registry with metadata (name, description, required permissions)
- [ ] **Per-task tool scoping**
  - Blueprint specifies which tool subset is available for each task
  - Prevents context window pollution from irrelevant tools
  - Policy engine enforces tool access based on role + skill trust level
- [ ] **Static context layer**
  - Local codebase indexing (tree-sitter AST, symbol tables)
  - Configurable workspace roots and exclusion patterns
  - Incremental index updates on SIGHUP / file-watch events
- [ ] **Dynamic context layer**
  - RAG integration for documentation and knowledge bases
  - Conversation history with configurable retention
  - Structured retrieval from issue trackers and CI logs
- [ ] **Context window management**
  - Token budget per skill invocation
  - Priority-based context packing (most relevant context first)
  - Automatic summarisation of large contexts

## Phase 11 — Planning Layer (Planned)

**Goal:** Implement a blueprint-based planning system (inspired by Stripe's
blueprint pattern) that decomposes tasks into directed graphs of deterministic
and agent-driven steps.

- [ ] **Blueprint engine** — `crustyclaw-blueprint` crate
  - Directed acyclic graph of execution nodes
  - Node types: `Deterministic` (shell, file transform, API call) and `AgentLoop` (LLM-driven)
  - Blueprint definition via TOML or proc-macro DSL
- [ ] **Task decomposition**
  - Break incoming requests into sub-tasks with dependency ordering
  - Parallel execution of independent sub-tasks
  - Partial-result aggregation
- [ ] **CI budget**
  - Configurable max CI rounds per blueprint (default: 2)
  - Local pre-flight checks (lint, type-check, test heuristics) before CI
  - Budget exhaustion triggers escalation to review layer
- [ ] **Escalation thresholds**
  - Per-blueprint configurable failure limits
  - Escalation actions: pause, return to human, retry with different strategy
  - Full context preservation on escalation (branch, logs, partial work)
- [ ] **Blueprint library**
  - Built-in blueprints for common tasks (fix flaky test, implement from spec, migration)
  - User-defined custom blueprints

## Phase 12 — Execution Layer (Planned)

**Goal:** Implement the runtime that spins up isolated environments, executes
blueprint steps, and manages lifecycle from invocation to PR.

- [ ] **Devbox provisioning** — inspired by Stripe's 10-second warm-pool model
  - Warm pool of pre-configured sandbox instances
  - Configurable pool size and pre-loading (source code, dependencies, tools)
  - <10s provision time for warm starts
- [ ] **Skill execution runtime**
  - Execute blueprint nodes inside sandboxed environments
  - Stream stdout/stderr back to daemon for logging
  - Async cancellation support (SIGTERM propagation to sandbox)
- [ ] **CI integration**
  - Trigger Forgejo Actions / external CI after local pre-flight passes
  - Parse CI results and feed back into agent loop
  - Configurable CI provider adapters
- [ ] **Artifact collection**
  - Capture execution outputs (diffs, test results, logs)
  - Branch management (create, push, PR creation)
  - Structured result envelope for the review layer
- [ ] **Concurrency control**
  - Configurable max concurrent executions
  - Queue with priority ordering
  - Resource-aware scheduling (memory, CPU budget across sandboxes)

## Phase 13 — Review Layer (Planned)

**Goal:** Automated and human review gates that validate execution outputs
before they reach production.

- [ ] **Automated review gates**
  - Lint pass (formatting, clippy, custom linters)
  - Type-check pass (full cargo check)
  - Test pass (unit + integration, configurable coverage threshold)
  - Policy evaluation (RBAC check on affected resources)
  - Security scan (dependency audit, secret detection)
- [ ] **Human-in-the-loop (HITL) enforcement**
  - Configurable approval gates at any point in the pipeline
  - Signal-based approval: operator sends approve/reject via Signal
  - CLI/TUI approval: `crustyclaw review approve <id>`
  - Timeout-based escalation: auto-reject after configurable period
- [ ] **Review policies** (TOML-configurable)
  - `auto` — fully automated, no human approval required
  - `approve-before-merge` — human must approve the final PR
  - `approve-before-execute` — human must approve before sandbox execution
  - `approve-every-step` — human approves each blueprint node
  - Per-skill and per-trust-level policy overrides
- [ ] **Diff presentation**
  - Structured diff display in TUI (review panel)
  - Signal message with summary + diff link
  - CLI diff viewer with inline comments

## Phase 14 — Feedback Loop (Planned)

**Goal:** Closed-loop system that tracks outcomes, learns from failures, and
continuously improves agent performance with optional human oversight at every
stage.

- [ ] **Outcome tracking**
  - Track per-blueprint: success rate, CI rounds used, time-to-merge
  - Track per-skill: invocation count, failure rate, escalation rate
  - Persistent metrics store (append-only log or SQLite)
- [ ] **Failure analysis**
  - Classify failures: sandbox error, CI failure, review rejection, timeout
  - Capture full context on failure for post-mortem
  - Automatic retry with alternative strategy on retriable failures
- [ ] **Human feedback integration**
  - Capture review comments and rejection reasons
  - Structured feedback forms (Signal / CLI / TUI)
  - Feedback linked to specific blueprint + execution context
- [ ] **Continuous improvement loop**
  - Aggregate feedback into prompt improvement suggestions
  - Blueprint tuning: adjust CI budget, pre-flight checks based on historical success
  - Tool effectiveness scoring: which MCP tools contribute to success
- [ ] **Observability**
  - `tracing` spans for every layer (sandbox → context → plan → execute → review)
  - `tracing-flame` integration for performance analysis
  - Structured JSON logs for external monitoring (Grafana, Datadog)
  - TUI dashboard: real-time pipeline status, queue depth, success rates
- [ ] **Human-in-the-loop enforcement policy**
  - Global HITL policy: always, never, or configurable per trust level
  - Escalation chains: agent → senior engineer → team lead
  - Audit trail: every human decision recorded with timestamp and rationale
  - Compliance mode: mandatory HITL for regulated environments

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

## Key Crate Dependencies

| Crate | Purpose |
|-------|---------|
| `tokio` | Async runtime for daemon |
| `clap` | CLI argument parsing |
| `ratatui` + `crossterm` | TUI framework |
| `serde` + `toml` | Config serialization |
| `presage` or `libsignal` | Signal protocol |
| `syn` + `quote` + `proc-macro2` | Procedural macro infrastructure |
| `zeroize` | Sensitive memory clearing |
| `tracing` + `tracing-subscriber` | Structured logging |
| `tracing-flame` | Flamegraph generation from tracing spans |
| `test-log` | Automatic tracing in tests |
| `pretty_assertions` | Readable test diffs |
| `tempfile` | Auto-cleanup temp dirs in tests |
| `inventory` or `linkme` | Plugin registration |

# Feature Comparison: OpenClaw vs NanoClaw vs CrustyClaw

> **Date:** 2026-02-19
> **Purpose:** Competitive analysis and gap identification for CrustyClaw's next development phase,
> with emphasis on container isolation, gateway architecture, and security flaw remediation.

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Platform Overview](#2-platform-overview)
3. [Feature Matrix](#3-feature-matrix)
4. [Architecture Comparison](#4-architecture-comparison)
5. [Security Comparison](#5-security-comparison)
6. [OpenClaw Exposed Security Flaws](#6-openclaw-exposed-security-flaws)
7. [NanoClaw Container Isolation Model](#7-nanoclaw-container-isolation-model)
8. [CrustyClaw Remediation Strategy](#8-crustyclaw-remediation-strategy)
9. [Priority Focus Areas](#9-priority-focus-areas)

---

## 1. Executive Summary

**OpenClaw** is a feature-rich, 52+ module Node.js agent with 15+ messaging channels, a Gateway
control plane, browser automation, cron scheduling, and a massive skill ecosystem (5,700+ skills
on ClawHub). It is also a security disaster: 6 CVEs in its first month, 42,000+ exposed instances,
800+ malicious skills in its registry, OAuth tokens in plaintext, and no process isolation between
skills. Its breadth is its weakness — the shared-memory monolith cannot be audited or hardened.

**NanoClaw** is a 500-line TypeScript response to OpenClaw's security crisis. It trades every feature
except WhatsApp messaging, memory, scheduled jobs, and agent swarms for genuine OS-level container
isolation (Apple Container on macOS, Docker on Linux). Each chat gets its own VM. The codebase is
auditable by one person in 10 minutes. The trade-off: no extension model, no multi-channel
support, no operator tooling, no policy engine.

**CrustyClaw** is positioned to combine NanoClaw's security posture with OpenClaw's operational
depth — in a memory-safe language with compile-time security guarantees. The next phase should
focus on: (1) Apple Container integration for real VM isolation, (2) a gateway layer for
multi-channel routing, and (3) systematic remediation of every OpenClaw security flaw class.

---

## 2. Platform Overview

| Attribute | OpenClaw | NanoClaw | CrustyClaw |
|-----------|----------|----------|------------|
| **Language** | TypeScript (Node.js) | TypeScript (Node.js) | Rust (`#![deny(unsafe_code)]`) |
| **Codebase size** | 52+ modules, thousands of files | ~500 lines, 4 source files | 6 crates, ~37 source files |
| **Architecture** | Shared-memory monolith (single Gateway process) | Single process, per-chat containers | Async daemon, message bus, sandbox backends |
| **License** | Open source | MIT | MIT |
| **GitHub stars** | 140,000+ | 7,000+ | Early development |
| **First release** | January 2026 | January 31, 2026 | In progress |
| **Creator** | Peter Steinberger | Gavriel Cohen | CrustyClaw Team |
| **LLM integration** | Multi-provider (Claude, GPT, DeepSeek, local) | Claude only (Agent SDK) | Architecture-ready, provider-agnostic |
| **Primary user channel** | Multi-channel (15+ platforms) | WhatsApp only | Signal only (E2E encrypted) |
| **Operator interface** | Web UI, CLI, macOS app | None | CLI + TUI |
| **Extension model** | Skills (SKILL.md + ClawHub registry) | Fork-and-modify + skill scripts | Forgejo Actions (sandboxed CI/CD) |
| **Memory safety** | None (JavaScript runtime) | None (JavaScript runtime) | Compile-time (Rust ownership) |

---

## 3. Feature Matrix

### 3.1 Messaging Channels

| Channel | OpenClaw | NanoClaw | CrustyClaw |
|---------|:--------:|:--------:|:----------:|
| WhatsApp | Yes (Baileys) | Yes | No |
| Signal | Yes (signal-cli) | No | Yes (type-state adapter) |
| Telegram | Yes (grammY) | No | No |
| Discord | Yes (discord.js) | No | No |
| Slack | Yes (Bolt) | No | No |
| Google Chat | Yes | No | No |
| iMessage | Yes (BlueBubbles) | No | No |
| Microsoft Teams | Yes (extension) | No | No |
| Matrix | Yes (extension) | No | No |
| WebChat | Yes | No | No |
| macOS/iOS/Android | Yes (companion apps) | No | No |

### 3.2 Agent Capabilities

| Capability | OpenClaw | NanoClaw | CrustyClaw |
|------------|:--------:|:--------:|:----------:|
| LLM chat loop | Yes | Yes (Agent SDK) | Architecture-ready |
| Persistent memory | Yes (local files) | Yes (per-group CLAUDE.md) | Planned |
| Browser automation | Yes (Chrome control) | No | No |
| File system access | Yes (unrestricted) | Yes (container-scoped) | Via sandbox |
| Shell execution | Yes (host-level) | Yes (container-scoped) | Via sandbox |
| Cron / scheduled jobs | Yes (6-field cron) | Yes (task scheduler) | No |
| Voice / speech | Yes (ElevenLabs) | No | No |
| Agent swarms | No | Yes (Agent SDK) | No |
| Multi-agent routing | Yes (per-channel agents) | No (per-group isolation) | Planned |
| Skill ecosystem | Yes (5,700+ on ClawHub) | Fork-modify pattern | Forgejo Actions |
| Canvas / visual workspace | Yes (A2UI) | No | No |
| Nodes / companion devices | Yes (camera, screen, GPS) | No | No |
| Webhooks | Yes | No | No |

### 3.3 Operator & Control Plane

| Feature | OpenClaw | NanoClaw | CrustyClaw |
|---------|:--------:|:--------:|:----------:|
| CLI | Yes | No | Yes (clap) |
| TUI | No | No | Yes (ratatui) |
| Web dashboard | Yes (Control UI) | No | No |
| WebSocket control plane | Yes (typed WS API) | No | No |
| Configuration system | JSON (`openclaw.json`) | JSON (mount-allowlist) | TOML (validated, typed) |
| Policy engine / RBAC | No (tool allow/deny lists) | No | Yes (priority rules, wildcards) |
| Live log streaming | No (file-based) | No | Yes (tracing → TUI) |
| Build metadata | No | No | Yes (git hash, timestamp, profile) |
| Health monitoring | Yes (heartbeat) | No | Planned |

### 3.4 Security Features

| Feature | OpenClaw | NanoClaw | CrustyClaw |
|---------|:--------:|:--------:|:----------:|
| Memory safety | No | No | Yes (Rust) |
| `unsafe` code ban | N/A | N/A | Yes (`#![deny(unsafe_code)]`) |
| Container isolation | Optional Docker sandbox | Apple Container / Docker | Sandbox backend trait |
| VM-level isolation | No | Yes (Apple Container) | Architecture-ready |
| Process isolation for skills | No (shared memory) | Yes (per-chat container) | Yes (Forgejo Actions) |
| Credential encryption | No (plaintext JSON) | File-based (container-scoped) | `#[derive(SecureZeroize)]` |
| Auth state machine | No | No | Yes (type-state, compile-time) |
| Rate limiting | No | No | Yes (token bucket per sender) |
| Input validation | Zod schemas | Minimal | `#[derive(Validate)]` |
| Policy engine | Tool allow/deny lists | None | RBAC with priority + wildcards |
| Supply chain scanning | No | No | Yes (cargo-audit, cargo-deny) |
| Fuzz testing | No | No | Yes (cargo-fuzz) |
| SBOM generation | No | No | Yes (CycloneDX) |
| Threat model | No | No | Yes (documented) |
| Sensitive field redaction | No | No | Yes (`#[derive(Redact)]`) |
| Compile-time key size checks | No | No | Yes (const assertions) |
| TLS version enforcement | No | No | Yes (const assertions) |
| Skill/extension vetting | VirusTotal (reactive) | N/A (fork-modify) | Forgejo sandbox (proactive) |

---

## 4. Architecture Comparison

### 4.1 OpenClaw: Shared-Memory Gateway

```
         ┌──────────────────────────────────────────────┐
         │            Single Node.js Process             │
         │                                               │
         │  ┌─────────┐ ┌────────┐ ┌────────┐          │
         │  │WhatsApp  │ │Telegram│ │ Slack  │ ... (15+)│
         │  │ Baileys  │ │ grammY │ │  Bolt  │          │
         │  └────┬─────┘ └───┬────┘ └───┬────┘          │
         │       └───────────┼─────────┘                │
         │                   ▼                          │
         │         ┌──────────────────┐                 │
         │         │  Gateway Router  │                 │
         │         │  (sessions, WS)  │                 │
         │         └────────┬─────────┘                 │
         │                  ▼                           │
         │  ┌───────────────────────────────┐           │
         │  │  Agent Loop (shared memory)   │           │
         │  │  skills, tools, memory, cron  │ ← NO ISOLATION
         │  └───────────────────────────────┘           │
         └──────────────────────────────────────────────┘
```

**Problem:** Every module, skill, and channel adapter runs in the same V8 heap. A malicious
skill can read any other skill's data, access credentials, or corrupt the agent loop. The
52+ modules share a single trust domain.

### 4.2 NanoClaw: Container-Per-Chat

```
         ┌─────────────┐
         │  Node.js     │
         │  Orchestrator│
         │  (~500 LOC)  │
         └──────┬───────┘
                │
        ┌───────┼───────┐
        ▼       ▼       ▼
   ┌─────────┐┌─────────┐┌─────────┐
   │ Container││Container││Container│
   │ (Chat A) ││(Chat B) ││(Chat C) │
   │ own FS   ││ own FS  ││ own FS  │
   │ own agent││own agent││own agent│
   └─────────┘└─────────┘└─────────┘
   Apple Container VMs (own kernel)
```

**Strength:** Genuine VM isolation per chat. Compromise of one container cannot affect
others or the host.

**Weakness:** No operator interface, no policy engine, no multi-channel support, no
extension model beyond fork-and-modify. The orchestrator itself is unaudited TypeScript
with no memory safety guarantees.

### 4.3 CrustyClaw: Daemon + Sandbox Backends

```
         ┌───────────────────────────────────┐
         │  Rust Daemon (memory-safe)        │
         │  ┌────────────────────────┐       │
         │  │  Message Bus (bounded) │       │
         │  └──┬────────┬────────┬──┘       │
         │     │        │        │          │
         │  ┌──┴──┐ ┌───┴──┐ ┌──┴──────┐   │
         │  │Auth │ │Policy│ │  Skill   │   │
         │  │ FSM │ │Engine│ │ Registry │   │
         │  └─────┘ └──────┘ └────┬────┘   │
         └──────────────────────── │ ────────┘
                                   │
              ┌────────────────────┼────────────────────┐
              ▼                    ▼                    ▼
      ┌──────────────┐   ┌──────────────┐   ┌──────────────┐
      │ Apple VZ      │   │ Linux NS     │   │ Forgejo      │
      │ (VM sandbox)  │   │ (cgroup)     │   │ Actions      │
      └──────────────┘   └──────────────┘   └──────────────┘
```

**Strength:** Memory-safe core, compile-time security guarantees, pluggable sandbox
backends, proper operator tooling, policy engine.

**Gap:** Container backends are architecturally defined but not yet connected to real
isolation runtimes. The Apple VZ backend needs integration with Apple's
Virtualization.framework (or the new Apple Container runtime). Gateway/multi-channel
routing is not yet implemented.

---

## 5. Security Comparison

### 5.1 Vulnerability Class Coverage

| Vulnerability Class | OpenClaw | NanoClaw | CrustyClaw |
|---------------------|----------|----------|------------|
| **Memory corruption** (buffer overflow, use-after-free) | Exposed (V8 + native addons) | Exposed (V8) | Eliminated (Rust ownership) |
| **Cross-site WebSocket hijacking** | CVE-2026-25253 (patched) | Not applicable (no WS) | Not applicable (no WS) |
| **SSRF** | CVE-2026-25593 (patched) | Not applicable | Not applicable |
| **Path traversal** | CVE-2026-25475 (patched) | Mitigated (container FS) | Mitigated (sandbox FS mounts) |
| **Auth bypass** | 93.4% of exposed instances | No auth model | Type-state auth (compile-time) |
| **Token exfiltration** | CVE-2026-25253 (query params) | No token model | `SecureZeroize` + `Redact` |
| **Credential plaintext storage** | Yes (JSON files) | Container-scoped files | `SecureZeroize` on Drop |
| **Sandbox escape** | CVE-2026-24763 (Docker bypass) | Apple Container VM (strong) | Backend-dependent |
| **Supply chain (malicious extensions)** | 800+ malicious skills on ClawHub | N/A (no registry) | Forgejo Actions (sandboxed) |
| **Prompt injection** | No mitigation | No mitigation | Skill sandboxing (partial) |
| **Denial of service** | No rate limiting | Per-group queue | Token bucket per sender |
| **Data leakage in logs** | Credentials in logs | Minimal logging | `#[derive(Redact)]` |
| **Privilege escalation** | No RBAC | No RBAC | Policy engine with default-deny |

### 5.2 Attack Surface Size

| Metric | OpenClaw | NanoClaw | CrustyClaw |
|--------|----------|----------|------------|
| Lines of code | ~50,000+ | ~500 | ~5,000 |
| npm/cargo dependencies | 45+ direct, hundreds transitive | Minimal | 14 direct, audited |
| Network listeners | HTTP + WS on configurable port | None (WhatsApp polling) | Configurable (daemon port) |
| Exposed instances (Internet) | 42,665 found, 5,194 confirmed vulnerable | None (local only) | None (local only, by design) |
| Skill registries (untrusted code) | ClawHub (20% malicious at peak) | None | Forgejo (self-hosted, sandboxed) |

---

## 6. OpenClaw Exposed Security Flaws

This section catalogs OpenClaw's known security issues so CrustyClaw can systematically
address each class.

### 6.1 CVE Registry

| CVE | Severity | Class | Description | CrustyClaw Mitigation |
|-----|----------|-------|-------------|----------------------|
| CVE-2026-25253 | Critical (CVSS 8.8) | CWE-669 (Incorrect Resource Transfer) | 1-click RCE via WebSocket hijacking — `gatewayUrl` query param auto-connects WS without origin validation, leaking auth tokens | No WebSocket gateway. Signal is the only user channel. Daemon listens on localhost only. Auth requires type-state FSM. |
| CVE-2026-25157 | High | Auth bypass | Authentication bypass in Gateway | Type-state auth: `Unauthenticated → Authenticated → Authorized`. Cannot reach authorized state without completing the chain (compile-time enforced). |
| CVE-2026-24763 | High | Sandbox escape | Docker sandbox bypass after incomplete CVE-2026-25253 fix | Apple Container VM isolation (own kernel, not namespace isolation). Sandbox backend trait with platform-specific implementations. |
| CVE-2026-25593 | Medium | SSRF | Server-side request forgery via Gateway | No user-facing HTTP endpoints. Daemon does not proxy requests. LLM calls use explicit provider URLs only. |
| CVE-2026-25475 | Medium | Path traversal | File access outside intended directories | Sandbox mounts are declarative (`SandboxConfig.mounts_ro`, `mounts_rw`). No host FS access from skill containers. |
| (Unnamed) | High | Missing auth | Multiple endpoints without authentication | All daemon operations go through policy engine. Default effect: deny. |

### 6.2 Architectural Flaws (Not CVE-Assigned)

| Flaw | OpenClaw Impact | CrustyClaw Remediation |
|------|----------------|----------------------|
| **Shared-memory monolith** | All 52+ modules share V8 heap. Malicious skill reads any data. | Skills run in isolated Forgejo Action containers. No shared memory. |
| **Plaintext credential storage** | OAuth tokens in `~/.openclaw/openclaw.json` without encryption | `#[derive(SecureZeroize)]` zeros memory on Drop. `KeyBuffer<N>` for sized key management. |
| **No origin validation on WebSocket** | Any website can connect to Gateway WS | No WebSocket endpoint. Signal E2E encryption for user channel. |
| **Skill registry poisoning** | 800+ malicious skills (20% of ClawHub) | Self-hosted Forgejo Actions. No public registry. Extensions are version-controlled workflows, not downloaded scripts. |
| **No rate limiting** | Resource exhaustion via message floods | Token-bucket rate limiter per sender with configurable burst and refill. |
| **Credential leakage in logs** | API keys appear in debug output | `#[derive(Redact)]` auto-redacts `#[redact]` fields in Debug impl. |
| **No supply chain scanning** | 45+ npm deps, no audit in CI | `cargo-audit` + `cargo-deny` in CI. `Cargo.lock` committed. SBOM generation on release. |
| **No RBAC / policy engine** | Tool allow/deny lists only (no roles, no priorities) | Full RBAC with role × action × resource rules, priority ordering, wildcard matching, default-deny. |
| **512 audit findings** | January 2026 audit found 512 vulnerabilities, 8 critical | Threat model documented. Compile-time invariants. Fuzz testing. |

### 6.3 Internet Exposure Data

- **42,665** publicly exposed OpenClaw instances found (researcher Maor Dayan)
- **5,194** actively verified as vulnerable
- **93.4%** exhibited authentication bypass conditions
- **549** instances correlated with known threat actor infrastructure (Kimsuky, APT28)
- **17,500+** instances vulnerable to CVE-2026-25253 specifically (Hunt.io)

**CrustyClaw design response:** The daemon binds to `127.0.0.1` by default. There is no
web-accessible endpoint. The user channel (Signal) does not require exposing the daemon to
the Internet. Operator access is via local CLI/TUI only.

---

## 7. NanoClaw Container Isolation Model

NanoClaw's primary innovation is OS-level container isolation. CrustyClaw should adopt and
extend this model.

### 7.1 Apple Container Architecture

| Property | NanoClaw Implementation | CrustyClaw Target |
|----------|------------------------|-------------------|
| **Runtime** | Apple Container (macOS Tahoe) | Apple Virtualization.framework via `SandboxBackend` trait |
| **Isolation level** | Full VM (own Linux kernel) | Full VM (own kernel) |
| **Per-chat isolation** | Yes (1 container per WhatsApp group) | Yes (1 container per skill execution) |
| **Filesystem** | Only explicitly mounted dirs visible | Declarative `SandboxConfig.mounts_ro` / `mounts_rw` |
| **Network** | Container cannot access host network | `NetworkPolicy::None` / `HostOnly` / `OutboundOnly` / allow-list |
| **IPC** | Per-group IPC directories, no cross-group access | Message bus envelopes (no shared memory between skills) |
| **Resource limits** | Implicit (container defaults) | Explicit: CPU cores, memory ceiling, timeout, max files, max procs |
| **Linux fallback** | Docker (namespace isolation, weaker) | `LinuxNamespaceBackend` (cgroups + namespaces) |
| **Mount validation** | `validateAdditionalMounts()` against allowlist | `SandboxConfig` validated at config load time |

### 7.2 What CrustyClaw Should Adopt

1. **Per-execution VM isolation** — Each skill invocation gets its own container with its own
   kernel. This is the strongest isolation model available on consumer hardware.

2. **Filesystem mount allowlists** — Skills only see directories that are explicitly mounted.
   No implicit access to host filesystem.

3. **No network by default** — Container network policy defaults to `none`. Skills that need
   network access must declare it and have it approved by the policy engine.

### 7.3 What CrustyClaw Should Improve Over NanoClaw

1. **Policy-gated resource allocation** — NanoClaw uses implicit container defaults.
   CrustyClaw should allow the policy engine to approve/deny specific resource requests
   (e.g., "this skill needs 2GB RAM and outbound HTTPS").

2. **Multiple backend support** — NanoClaw is Apple Container or Docker. CrustyClaw's
   `SandboxBackend` trait already supports pluggable backends. Add real implementations
   for Apple Virtualization.framework, Linux namespaces, and potentially Firecracker for
   server deployments.

3. **Operator visibility** — NanoClaw provides no visibility into container state.
   CrustyClaw's TUI should show running containers, resource usage, and isolation status.

4. **Audit trail** — Every container launch, mount request, and network policy decision
   should be logged via `tracing` and visible in the TUI logs panel.

---

## 8. CrustyClaw Remediation Strategy

For each OpenClaw vulnerability class, CrustyClaw's defense:

| OpenClaw Flaw | CrustyClaw Defense | Layer | Status |
|---------------|-------------------|-------|--------|
| Memory corruption | Rust ownership + `#![deny(unsafe_code)]` | Language | Enforced |
| WebSocket hijacking (CVE-2026-25253) | No WebSocket endpoint. Signal-only user channel. | Architecture | By design |
| Auth bypass (CVE-2026-25157) | Type-state auth FSM (compile-time) | Core | Implemented |
| Sandbox escape (CVE-2026-24763) | VM-level isolation (Apple VZ), not namespace-only | Isolation | Architecture-ready |
| SSRF (CVE-2026-25593) | No HTTP proxy. Explicit LLM provider URLs only. | Architecture | By design |
| Path traversal (CVE-2026-25475) | Declarative FS mounts in `SandboxConfig` | Isolation | Implemented |
| Plaintext credentials | `SecureZeroize` + `KeyBuffer<N>` + `Redact` | Security | Implemented |
| Credential log leakage | `#[derive(Redact)]` on sensitive structs | Security | Implemented |
| Supply chain poisoning | Self-hosted Forgejo, `cargo-audit`, `cargo-deny`, SBOM | Build | CI-enforced |
| No rate limiting | Token-bucket per sender | Signal | Implemented |
| No RBAC | Policy engine (role × action × resource, priorities, wildcards) | Config | Implemented |
| Internet exposure | Bind `127.0.0.1` default, no web endpoints | Architecture | By design |

---

## 9. Priority Focus Areas

Based on this analysis, CrustyClaw's next development phase should focus on three areas:

### 9.1 Apple Container Integration (High Priority)

**Goal:** Connect the existing `SandboxBackend` trait to real Apple Virtualization.framework
for VM-level skill isolation on macOS.

**Work required:**
- Implement `AppleVzBackend` using Apple's Virtualization.framework Rust bindings
  (or FFI to the Swift/ObjC API)
- Container image management (pull, cache, lifecycle)
- Mount translation from `SandboxConfig` to Virtualization.framework shared directories
- Network policy enforcement via virtual network device configuration
- Resource limit mapping (CPU, memory) to VM configuration
- Graceful container shutdown with timeout enforcement
- TUI panel for container status and resource usage

**Acceptance criteria:**
- Skills execute inside Apple Container VMs with their own kernel
- Host filesystem is invisible except for explicitly mounted paths
- Container crash does not affect daemon or other containers
- Resource limits are enforced (CPU, memory, timeout)
- `NetworkPolicy::None` results in no network device attached to VM

### 9.2 Gateway Layer (Medium Priority)

**Goal:** Add a lightweight message routing layer that can support multiple inbound channels
while maintaining the security model.

**Work required:**
- Define a `Channel` trait for message sources (Signal is the first implementation)
- Session routing: map `(channel, sender, group)` tuples to isolated agent contexts
- Per-session skill isolation (each session gets its own container scope)
- Extend the policy engine to support channel-level rules
  (e.g., `allow signal-user read messages; deny telegram-user execute shell`)
- Gateway does NOT expose HTTP/WS endpoints — channels connect outward, not inward

**Design constraint:** Unlike OpenClaw's Gateway (which listens on a network port),
CrustyClaw's routing layer should be an internal daemon component. Channels are adapters
that poll external services (Signal polls for messages). No inbound network listener is
required for the gateway itself.

**Acceptance criteria:**
- Multiple channels can route messages through the daemon simultaneously
- Each channel × sender combination maps to an isolated session
- Policy rules can discriminate by channel and sender role
- Adding a new channel requires implementing one trait, not modifying the router

### 9.3 Security Flaw Remediation Completeness (Ongoing)

**Goal:** Ensure every OpenClaw vulnerability class documented in Section 6 has a tested,
verified mitigation in CrustyClaw.

**Work required:**
- Integration tests for each CVE-equivalent scenario
- Fuzz targets for all input boundaries (config, Signal messages, policy rules)
- Penetration test plan covering: auth bypass, sandbox escape, credential leakage,
  supply chain, rate limiting bypass, path traversal
- Document residual risks in `THREAT_MODEL.md` for any classes that cannot be fully
  mitigated at the application layer

**Acceptance criteria:**
- Every row in Section 5.1 has a passing integration test
- Fuzz targets run ≥ 1M iterations without panics
- Threat model is updated with CVE-equivalent analysis
- No `unsafe` code exists in any crate without documented justification

---

## References

### OpenClaw
- [OpenClaw GitHub Repository](https://github.com/openclaw/openclaw)
- [OpenClaw Architecture Documentation](https://docs.openclaw.ai/concepts/architecture)
- [OpenClaw Tools Documentation](https://docs.openclaw.ai/tools)
- [OpenClaw Skills Documentation](https://docs.openclaw.ai/tools/skills)
- [CVE-2026-25253 Analysis (SOCRadar)](https://socradar.io/blog/cve-2026-25253-rce-openclaw-auth-token/)
- [Six New OpenClaw Vulnerabilities (Infosecurity Magazine)](https://www.infosecurity-magazine.com/news/researchers-six-new-openclaw/)
- [OpenClaw Security Crisis (Conscia)](https://conscia.com/blog/the-openclaw-security-crisis/)
- [OpenClaw Security Guide (Adversa AI)](https://adversa.ai/blog/openclaw-security-101-vulnerabilities-hardening-2026/)
- [OpenClaw Is a Security Nightmare (Barrack.ai)](https://blog.barrack.ai/openclaw-security-vulnerabilities-2026/)
- [OpenClaw Vulnerability Notification (University of Toronto)](https://security.utoronto.ca/advisories/openclaw-vulnerability-notification/)

### NanoClaw
- [NanoClaw GitHub Repository](https://github.com/qwibitai/nanoclaw)
- [NanoClaw Architecture Analysis (Sudheer Singh)](https://fumics.in/posts/2026-02-02-nanoclaw-agent-architecture)
- [NanoClaw vs OpenClaw Security (VentureBeat)](https://venturebeat.com/orchestration/nanoclaw-solves-one-of-openclaws-biggest-security-issues-and-its-already)
- [NanoClaw Apple Container Isolation (PulsarTech)](https://pulsartech.news/en/articles/show-hn-nanoclaw-clawdbot-in-500-lines-of-ts-with-apple-container-isolation-ml4elrd6)

### General
- [DigitalOcean: What is OpenClaw](https://www.digitalocean.com/resources/articles/what-is-openclaw)
- [OpenClaw Wikipedia](https://en.wikipedia.org/wiki/OpenClaw)

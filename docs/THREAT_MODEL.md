# CrustyClaw Threat Model

## System Overview

CrustyClaw is a security-first AI agent daemon that processes user messages
via Signal and executes skills (LLM-powered actions). Operators manage it
through CLI and TUI interfaces.

## Trust Boundaries

```
┌─────────────────────────────────────────┐
│  Operator Zone (trusted)                │
│  ┌───────┐  ┌───────┐                  │
│  │  CLI  │  │  TUI  │                  │
│  └───┬───┘  └───┬───┘                  │
│      └─────┬────┘                       │
│            ▼                            │
│  ┌─────────────────┐                   │
│  │   Core Daemon    │ ◄── Trust Boundary│
│  │  (message bus)   │                   │
│  └────┬────┬───┬───┘                   │
│       │    │   │                        │
└───────┼────┼───┼────────────────────────┘
        │    │   │
        ▼    │   ▼
┌───────────┐│ ┌──────────────────┐
│  Signal   ││ │  Forgejo Actions │
│  Channel  ││ │  (sandboxed)     │
│ (E2E enc) ││ └──────────────────┘
└───────────┘│
             ▼
     ┌──────────────┐
     │  LLM Provider │
     │  (external)   │
     └──────────────┘
```

## Assets

| Asset | Sensitivity | Location |
|-------|------------|----------|
| Signal credentials | Critical | `data/signal/` |
| LLM API keys | Critical | Config / env vars |
| User messages | High | In-flight (message bus) |
| Policy rules | High | `crustyclaw.toml` |
| Skill execution results | Medium | In-flight |
| Configuration | Medium | `crustyclaw.toml` |
| Logs | Low-Medium | Stdout / log files |

## Threats and Mitigations

### T1: Memory Safety Vulnerabilities
- **Threat:** Buffer overflows, use-after-free, data races
- **Mitigation:** `#![deny(unsafe_code)]` across all crates, Rust's ownership model
- **Status:** Enforced

### T2: Supply Chain Attacks
- **Threat:** Compromised dependencies introducing backdoors
- **Mitigation:** `cargo-audit` in CI, `Cargo.lock` committed, dependency review
- **Status:** Implemented (CI audit job)

### T3: Sensitive Data in Logs/Debug Output
- **Threat:** API keys or credentials leaking through log output
- **Mitigation:** `#[derive(Redact)]` on sensitive structs, `#[derive(SecureZeroize)]` for memory clearing
- **Status:** Implemented

### T4: Unauthorized Access to Daemon
- **Threat:** Unprivileged users executing admin actions
- **Mitigation:** Policy engine (RBAC), type-state auth lifecycle
- **Status:** Implemented

### T5: Message Injection / Abuse
- **Threat:** Malicious users flooding the system or injecting crafted messages
- **Mitigation:** Token-bucket rate limiter per sender, input validation (`#[derive(Validate)]`)
- **Status:** Implemented

### T6: Signal Protocol Compromise
- **Threat:** Man-in-the-middle or session hijacking
- **Mitigation:** Type-state adapter (Unlinked→Linked→Verified), E2E encryption via Signal protocol
- **Status:** Type-state enforced, protocol integration deferred

### T7: Skill Execution Escape
- **Threat:** Malicious or buggy skills breaking out of sandbox
- **Mitigation:** Forgejo Actions run in isolated containers, no direct daemon access
- **Status:** Architecture enforced, container isolation deferred

### T8: Configuration Tampering
- **Threat:** Unauthorized modification of config or policy rules
- **Mitigation:** Config validation on load, file permission checks (future)
- **Status:** Validation implemented

### T9: Denial of Service
- **Threat:** Resource exhaustion from message floods or skill loops
- **Mitigation:** Rate limiter, bounded message bus, bounded log buffer
- **Status:** Implemented

### T10: Credential Persistence in Memory
- **Threat:** Sensitive data remaining in memory after use
- **Mitigation:** `#[derive(SecureZeroize)]` with `zeroize` crate, `KeyBuffer<N>` with const-size enforcement
- **Status:** Implemented

## Security Invariants

These are enforced at compile time:

1. **No unsafe code** — `#![deny(unsafe_code)]` in all crates
2. **Minimum key sizes** — `security::assert_key_size::<N>()` rejects < 256-bit keys
3. **TLS version floor** — `security::assert_tls_version::<V>()` rejects < TLS 1.2
4. **Auth state machine** — Cannot access authorized resources without going through
   `Unauthenticated → Authenticated → Authorized`
5. **Signal lifecycle** — Cannot send messages without `Unlinked → Linked → Verified`

## Residual Risks

- LLM prompt injection (mitigated by skill sandboxing, not fully addressed)
- Physical access to host system
- Signal protocol library vulnerabilities (upstream dependency)
- Side-channel attacks (not in scope for application-layer defense)

# Security Guide

CrustyClaw is designed as a security-first system. This document describes the
security features, how to use them, and the security invariants that are enforced
at compile time.

## Compile-time invariants

These are enforced by the Rust compiler — they cannot be bypassed at runtime:

1. **No unsafe code** — `#![deny(unsafe_code)]` in all crates. Any `unsafe`
   block causes a compile error.
2. **Minimum key sizes** — `security::assert_key_size::<N>()` is a const
   assertion that rejects keys shorter than 256 bits.
3. **TLS version floor** — `security::assert_tls_version::<V>()` rejects
   TLS versions below 1.2 at compile time.
4. **Auth state machine** — the type-state pattern on `AuthSession` makes it
   impossible to access authorized resources without completing the full
   `Unauthenticated -> Authenticated -> Authorized` lifecycle.
5. **Signal lifecycle** — `SignalAdapter` enforces `Unlinked -> Linked ->
   Verified` at the type level.

## Derive macros

### `#[derive(Redact)]`

Redacts fields marked `#[redact]` in `Debug` output, replacing their values
with `[REDACTED]`.

```rust
#[derive(Redact)]
struct Credentials {
    pub username: String,
    #[redact]
    pub api_key: String,
}
// Debug output: Credentials { username: "alice", api_key: [REDACTED] }
```

### `#[derive(SecureZeroize)]`

Ensures sensitive data is zeroed in memory when the struct is dropped, using
the `zeroize` crate.

```rust
#[derive(SecureZeroize)]
struct Secret {
    pub key_material: Vec<u8>,
}
// Memory is zeroed when `Secret` is dropped
```

### `#[derive(Validate)]`

Adds field-level validation rules checked at runtime.

```rust
#[derive(Validate)]
struct Config {
    #[validate(non_empty)]
    pub host: String,
    #[validate(range(min = 1, max = 65535))]
    pub port: u16,
    #[validate(min_len = 32)]
    pub api_key: String,
}
```

## Policy engine (RBAC)

Access control is configured in `crustyclaw.toml` under `[policy]`. Rules are
evaluated by priority (highest first). The first matching rule determines the
outcome.

```toml
[policy]
default_effect = "deny"

[[policy.rules]]
role = "admin"
action = "*"
resource = "*"
effect = "allow"
priority = 100
```

Check a policy decision via CLI:

```bash
crustyclaw-cli policy --role user --action write --resource secrets
```

## Rate limiting

The Signal adapter applies per-sender token-bucket rate limiting to prevent
message flooding. Configure via the `RateLimitConfig` struct (defaults: 10
tokens, 60-second refill interval).

## Sandbox isolation

Skills execute inside sandboxed environments. Three backends are available:

| Backend | Platform | Isolation level |
|---------|----------|-----------------|
| `apple-vz` | macOS | Full VM (Apple Virtualization.framework) |
| `linux-ns` | Linux | Namespaces + seccomp + Landlock + cgroups |
| `noop` | Any | None (development/testing only) |

Sandbox parameters (memory, CPU, timeout, network) are configured in
`[isolation]`. See [configuration.md](configuration.md) for details.

## Supply chain

- `Cargo.lock` is committed to the repository
- `cargo-audit` runs in CI to detect known vulnerabilities
- `cargo-deny` checks licenses and duplicate dependencies
- MSRV is pinned and tested in CI

## Threat model

See [THREAT_MODEL.md](THREAT_MODEL.md) for the full threat model covering all
10 identified threat categories and their mitigations.

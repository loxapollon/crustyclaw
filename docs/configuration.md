# Configuration Reference

CrustyClaw is configured via a TOML file, by default `crustyclaw.toml` in the
working directory. All sections and keys are optional — defaults are applied for
any omitted values.

## `[daemon]`

Core daemon settings.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `listen_addr` | string | `"127.0.0.1"` | Address the daemon listens on for control-plane connections |
| `listen_port` | u16 | `9100` | Port the daemon listens on (must be non-zero) |

## `[signal]`

Signal messaging channel settings.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `enabled` | bool | `false` | Whether the Signal channel is active |
| `data_dir` | string | `"data/signal"` | Path to the Signal data directory |

## `[logging]`

Log output configuration.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `level` | string | `"info"` | Log level filter: `"trace"`, `"debug"`, `"info"`, `"warn"`, `"error"` |

## `[isolation]`

Sandbox / isolation configuration for skill execution.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `backend` | string | `"auto"` | Backend: `"auto"`, `"apple-vz"`, `"linux-ns"`, `"noop"` |
| `default_memory_bytes` | u64 | `268435456` (256 MiB) | Memory limit per sandbox (must be non-zero) |
| `default_cpu_fraction` | f64 | `0.5` | CPU fraction per sandbox, range (0.0, 1.0] |
| `default_timeout_secs` | u64 | `60` | Execution timeout in seconds (0 = no timeout) |
| `default_network` | string | `"none"` | Network policy: `"none"`, `"host-only"`, `"outbound-only"` |
| `max_concurrent` | usize | `4` | Maximum concurrent sandboxes (must be >= 1) |

### Backend selection

- **`auto`** — picks the best available backend for the platform (Apple VZ on macOS, Linux NS on Linux, falls back to noop)
- **`apple-vz`** — Apple Virtualization.framework (macOS only)
- **`linux-ns`** — Linux namespaces + seccomp + Landlock
- **`noop`** — no-op backend (no isolation, always available; for development/testing)

## `[policy]`

Role-based access control settings.

| Key | Type | Default | Description |
|-----|------|---------|-------------|
| `default_effect` | string | `"deny"` | Default policy when no rule matches: `"allow"` or `"deny"` |
| `rules` | array | `[]` | Policy rules (see below) |

### `[[policy.rules]]`

Each rule evaluates access for a given (role, action, resource) triple.

| Key | Type | Required | Description |
|-----|------|----------|-------------|
| `role` | string | yes | Role to match (e.g. `"admin"`, `"user"`, `"*"` for any) |
| `action` | string | yes | Action to match (e.g. `"read"`, `"write"`, `"*"` for any) |
| `resource` | string | yes | Resource to match (e.g. `"config"`, `"skills"`, `"*"` for any) |
| `effect` | string | yes | `"allow"` or `"deny"` |
| `priority` | u32 | no | Higher priority rules are evaluated first (default: 0) |

## Config reload (SIGHUP)

When the daemon receives `SIGHUP`, it re-reads the config file from disk
asynchronously. The new config is published via a `tokio::sync::watch` channel:

- Running skills are **never** interrupted.
- Consumers pick up the new config at their next natural pause / compaction point.
- If the new file fails validation, the current config is kept and an error is logged.

## Full example

```toml
[daemon]
listen_addr = "0.0.0.0"
listen_port = 8080

[signal]
enabled = true
data_dir = "/var/lib/crustyclaw/signal"

[logging]
level = "debug"

[isolation]
backend = "linux-ns"
default_memory_bytes = 536870912  # 512 MiB
default_cpu_fraction = 0.25
default_timeout_secs = 120
default_network = "outbound-only"
max_concurrent = 8

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
resource = "config"
effect = "allow"
priority = 50

[[policy.rules]]
role = "user"
action = "write"
resource = "secrets"
effect = "deny"
priority = 90
```

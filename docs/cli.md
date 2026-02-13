# CLI Reference

The CrustyClaw CLI (`crustyclaw-cli`) provides subcommands for managing the
daemon, inspecting configuration, and evaluating security policies.

## Global options

| Flag | Description |
|------|-------------|
| `-c, --config <PATH>` | Path to config file (default: `crustyclaw.toml`) |
| `-v, --verbose` | Increase log verbosity (`-v` = debug, `-vv` = trace) |
| `--help` | Show help |
| `--version` | Show version |

## Subcommands

### `start`

Start the CrustyClaw daemon. Runs until SIGTERM, SIGINT (Ctrl-C), or an
internal shutdown signal.

```bash
crustyclaw-cli start
crustyclaw-cli -c /etc/crustyclaw.toml start
```

The daemon responds to OS signals:

- **SIGHUP** — reload config from disk (non-interruptive to running skills)
- **SIGTERM / SIGINT** — graceful shutdown

### `stop`

Send a stop signal to a running daemon.

```bash
crustyclaw-cli stop
```

> Status: pending daemon IPC implementation.

### `status`

Query the status of a running daemon.

```bash
crustyclaw-cli status
```

> Status: pending daemon IPC implementation.

### `config`

Validate and display configuration.

```bash
# Validate and show summary
crustyclaw-cli config

# Dump the resolved config as TOML
crustyclaw-cli config --show
```

### `version`

Show build version, git hash, and build profile.

```bash
crustyclaw-cli version
```

### `policy`

Evaluate a policy access check against the loaded rules.

```bash
crustyclaw-cli policy --role admin --action write --resource secrets
```

Output shows `ALLOWED`, `DENIED`, or `NO MATCH (default deny)`.

### `plugins`

List registered Forgejo Action plugins.

```bash
crustyclaw-cli plugins
```

### `isolation`

Show isolation / sandbox configuration and backend status.

```bash
crustyclaw-cli isolation
```

Shows the configured backend, resolved backend, availability, and all default
sandbox parameters.

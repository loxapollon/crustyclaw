# Getting Started

## Prerequisites

- **Rust 1.93.0+** (edition 2024)
- A terminal emulator with UTF-8 and 256-color support (for the TUI)

## Installation from source

```bash
git clone <repository-url> crustyclaw
cd crustyclaw
cargo build --release
```

The binaries are placed in `target/release/`:

| Binary | Description |
|--------|-------------|
| `crustyclaw-cli` | CLI control plane |
| `crustyclaw-tui` | Interactive TUI |

## First run

### 1. Create a configuration file (optional)

CrustyClaw works with sensible defaults. To customize, create `crustyclaw.toml`
in your working directory:

```toml
[daemon]
listen_addr = "127.0.0.1"
listen_port = 9100

[logging]
level = "info"
```

See [configuration.md](configuration.md) for the full reference.

### 2. Start the daemon

```bash
# Via CLI
crustyclaw-cli start

# Or via CLI with custom config path
crustyclaw-cli -c /path/to/crustyclaw.toml start

# Or use the TUI for interactive operation
crustyclaw-tui
```

### 3. Verify it works

```bash
# Check daemon status
crustyclaw-cli status

# Show resolved configuration
crustyclaw-cli config --show

# Show build info
crustyclaw-cli version
```

## Next steps

- [Configuration reference](configuration.md) — full TOML reference
- [CLI reference](cli.md) — all CLI subcommands
- [TUI guide](tui.md) — panels and keybindings
- [Security guide](security.md) — threat model and security features
- [Extension guide](extensions.md) — writing Forgejo Action plugins

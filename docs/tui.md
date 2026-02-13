# TUI Guide

The CrustyClaw TUI (`crustyclaw-tui`) is an interactive terminal interface for
monitoring and managing the daemon. It renders a four-panel view with vim-style
keybindings.

## Starting the TUI

```bash
cargo run -p crustyclaw-tui
```

The TUI loads `crustyclaw.toml` from the working directory (falls back to
defaults if not found).

## Panels

### 1. Dashboard

Overview of daemon status:

- Listen address and port
- Signal channel status (enabled / disabled)
- Log level
- Isolation backend and availability
- Policy rule count
- Uptime

### 2. Logs

Live, scrollable log viewer. Captures all `tracing` events emitted by the
daemon and displays them with:

- Elapsed time since startup
- Log level (color-coded: red=ERROR, yellow=WARN, green=INFO, blue=DEBUG, gray=TRACE)
- Target module
- Message text

The panel auto-follows new entries by default. Scrolling up disables
auto-follow; scrolling to the bottom re-enables it.

### 3. Messages

Live view of inbound/outbound messages on the daemon message bus. Each entry
shows:

- Timestamp
- Direction arrow (`>>` inbound green, `<<` outbound cyan)
- Channel name
- Message body

Auto-follow behaviour is the same as the Logs panel.

### 4. Config

Displays the resolved configuration as syntax-highlighted TOML. Section headers,
keys, string values, numeric values, and booleans are color-coded.

## Keybindings

| Key | Action |
|-----|--------|
| `q` | Quit the TUI |
| `Tab` / `l` | Switch to next panel |
| `BackTab` / `h` | Switch to previous panel |
| `j` | Scroll down 1 line |
| `k` | Scroll up 1 line |
| `d` | Scroll down half page (10 lines) |
| `u` | Scroll up half page (10 lines) |
| `gg` | Scroll to top (two-key sequence) |
| `G` | Scroll to bottom |
| `1` | Jump to Dashboard |
| `2` | Jump to Logs |
| `3` | Jump to Messages |
| `4` | Jump to Config |

## Layout

```
┌─ CrustyClaw ───────────────────────────────────────┐
│ 1:Dashboard | 2:Logs | 3:Messages | 4:Config       │
└─────────────────────────────────────────────────────┘
┌─ Dashboard ─────────────────────────────────────────┐
│                                                     │
│  (active panel content)                             │
│                                                     │
└─────────────────────────────────────────────────────┘
 q:quit  Tab/l:next  BackTab/h:prev  j/k:scroll  ...
```

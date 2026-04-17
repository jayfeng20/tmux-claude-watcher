# tmux-monitor

A terminal UI that gives you a live, color-coded overview of every active tmux pane — what process is running, what it is doing, and how long it has been in that state.

Built with special awareness of [Claude Code](https://claude.ai/code): panes running `claude` are classified into fine-grained states (Thinking, Executing, Generating, Awaiting Input) so you can glance at the monitor and know exactly where each agent is in its work cycle.

![screenshot placeholder](docs/screenshot.png)

---

## Features

- **Live polling** — refreshes all panes every 2 seconds without blocking the UI
- **State classification** — distinguishes shell vs. Claude panes and their sub-states
- **Timing columns** — shows how long since a pane was last focused and how long it has been in its current state
- **Keyboard navigation** — scroll through panes with `j`/`k` or arrow keys
- **Non-intrusive logging** — writes to a rolling daily file in `/tmp` so stdout is never polluted while the TUI is active

---

## Pane states

### Claude panes

| Icon | State          | Meaning                              |
|------|----------------|--------------------------------------|
| `>_` | Awaiting Input | Input box visible, waiting for a prompt |
| `◉`  | Generating     | Streaming a text response            |
| `◌`  | Thinking       | Extended reasoning phase (spinner visible) |
| `⚙`  | Executing      | Tool use in progress                 |
| `○`  | Idle           | Open but no activity                 |
| `✗`  | Error          | Error output visible                 |

### Shell panes (`bash`, `zsh`, `fish`, `sh`)

| Icon | State          | Meaning                        |
|------|----------------|--------------------------------|
| `>_` | Awaiting Input | Prompt visible, ready for input |
| `◉`  | Processing     | Command running                |
| `○`  | Idle           | No content / inactive          |
| `✗`  | Error          | Error output on last line      |

---

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) 1.85 or later (edition 2024)
- [tmux](https://github.com/tmux/tmux) installed and a session must be running
- A terminal emulator with Unicode and 256-color support

---

## Installation

```bash
git clone https://github.com/jayfeng20/tmux-monitor.git
cd tmux-monitor
cargo build --release
```

The compiled binary will be at `target/release/claude-pane-monitor`.

Optionally, copy it somewhere on your `$PATH`:

```bash
cp target/release/claude-pane-monitor ~/.local/bin/
```

---

## Usage

Make sure at least one tmux session is running, then launch the monitor from inside (or outside) that session:

```bash
cargo run --release
# or, after installing:
claude-pane-monitor
```

### Key bindings

| Key         | Action              |
|-------------|---------------------|
| `q` / `Q`   | Quit                |
| `j` / `↓`   | Move selection down |
| `k` / `↑`   | Move selection up   |

### Logs

Structured logs are written to `/tmp/pane-monitor.YYYY-MM-DD` (one file per day). To stream them while the monitor is running:

```bash
tail -f /tmp/pane-monitor.$(date +%Y-%m-%d)
```

To control log verbosity, set `RUST_LOG` before launching:

```bash
RUST_LOG=debug claude-pane-monitor
```

---

## Running tests

```bash
cargo test
```

End-to-end tests are in `tests/e2e.rs`. Unit tests for pane parsing and state classification live alongside their source modules in `src/tmux/`.

---

## Project structure

```
src/
  main.rs               # Entry point: async event loop + polling task
  lib.rs                # Library root, re-exports the tmux module
  tmux/
    mod.rs              # Module declarations
    pane.rs             # PaneInfo, PaneState, and state classification logic
    pane_manager.rs     # Talks to tmux, owns the pane snapshot
    ui.rs               # Ratatui TUI: rendering and input handling
tests/
  e2e.rs                # Integration / end-to-end tests
```

---

## License

[MIT](LICENSE)

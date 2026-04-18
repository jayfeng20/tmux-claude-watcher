# tmux-monitor

A terminal UI that gives you a live, color-coded overview of every active tmux pane — what process is running, what it is doing, and how long it has been in that state.

Built with special awareness of [Claude Code](https://claude.ai/code): panes running `claude` are classified into fine-grained states (Thinking, Executing, Awaiting Input, Done) so you can glance at the monitor and know exactly where each agent is in its work cycle.

---

## Features

- **Live polling** — refreshes all panes every 2 seconds without blocking the UI
- **State classification** — distinguishes shell vs. Claude panes and their sub-states
- **Timing column** — shows how long each pane has been in its current state
- **Active column** — shows only panes that are truly receiving keyboard input (attached session + active window + active pane)
- **Keyboard navigation** — scroll through panes with `j`/`k` or arrow keys
- **Non-intrusive logging** — writes to a rolling daily file in `/tmp` so stdout is never polluted while the TUI is active

---

## Pane states

### Claude panes

| Icon | Color  | State               | Meaning                                          |
|------|--------|---------------------|--------------------------------------------------|
| `◌`  | orange | Thinking            | Extended reasoning in progress                   |
| `◑`  | yellow | Executing           | Generating a response or running a tool          |
| `❯`  | red    | Awaiting Input      | Input box visible, Claude is asking a question   |
| `!`  | red    | Awaiting Permission | Tool permission prompt needs approval            |
| `✓`  | green  | Done                | Task completed, input box visible, no question   |
| `?`  | dim    | Unknown             | State could not be determined                    |

### Shell panes (`bash`, `zsh`, `fish`, `sh`)

| Icon | Color | State          | Meaning                                        |
|------|-------|----------------|------------------------------------------------|
| `○`  | green | Idle           | Shell prompt visible (`%`, `$`, `#`) — ready   |
| `❯`  | red   | Awaiting Input | Process running or requesting input            |
| `✗`  | red   | Error          | Error output on the last line                  |

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

The compiled binary will be at `target/release/tmux-monitor`.

Optionally, copy it somewhere on your `$PATH`:

```bash
cp target/release/tmux-monitor ~/.local/bin/
```

---

## Usage

Make sure at least one tmux session is running, then launch the monitor from inside (or outside) that session:

```bash
cargo run --release
# or, after installing:
tmux-monitor
```

### Setting up tmux sessions to monitor

Create a new named session:

```bash
tmux new-session -s work
```

Create additional windows (tabs) inside a session:

```bash
tmux new-window -t work -n editor
tmux new-window -t work -n logs
```

Split a window into panes:

```bash
# Split horizontally (left/right)
tmux split-window -h -t work:editor

# Split vertically (top/bottom)
tmux split-window -v -t work:editor
```

Start Claude Code in a pane:

```bash
# From inside a tmux pane:
claude

# Or send the command to a specific pane (session:window.pane):
tmux send-keys -t work:editor.0 'claude' Enter
```

List all active panes across all sessions (what the monitor reads):

```bash
tmux list-panes -a
```

### Key bindings

| Key         | Action              |
|-------------|---------------------|
| `q` / `Q`   | Quit                |
| `j` / `↓`   | Move selection down |
| `k` / `↑`   | Move selection up   |
| `?`         | Toggle help panel   |
| `Esc`       | Close help panel    |

### Logs

Structured logs are written to `/tmp/pane-monitor.YYYY-MM-DD` (one file per day). To stream them while the monitor is running:

```bash
tail -f /tmp/pane-monitor.$(date +%Y-%m-%d)
```

To control log verbosity, set `RUST_LOG` before launching:

```bash
RUST_LOG=debug tmux-monitor
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
    pane/
      claude.rs         # ClaudeStatus detection from capture-pane content
      shell.rs          # ShellKind and ShellStatus detection
    pane_manager.rs     # Talks to tmux, owns the pane snapshot
    ui.rs               # Ratatui TUI: rendering and input handling
tests/
  e2e.rs                # Integration / end-to-end tests
```

---

## License

[MIT](LICENSE)

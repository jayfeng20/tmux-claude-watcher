# tmux-claude-watcher

A terminal UI that gives you a live, color-coded overview of every active tmux pane — what process is running, what it is doing, and how long it has been in that state.

Built with special awareness of [Claude Code](https://claude.ai/code): panes running `claude` are classified into fine-grained states (Thinking, Executing, Awaiting Input, Done) so you can glance at the monitor and know exactly where each agent is in its work cycle.

![Panel](examples/ui.png)
![Panel](examples/help.png)
---

## Features

- **Live polling** — refreshes all panes every 2 seconds without blocking the UI
- **State classification** — distinguishes shell vs. Claude panes and their sub-states
- **Timing column** — shows how long each pane has been in its current state
- **Active column** — shows only panes that are truly receiving keyboard input (attached session + active window + active pane)
- **Keyboard navigation** — scroll through panes with `j`/`k` or arrow keys; press `↵` to jump directly to a pane
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

### tc-watcher panes

| Icon | Color  | State  | Meaning                                                  |
|------|--------|--------|----------------------------------------------------------|
| `○`  | green  | Active | Monitor is running and polling pane states               |
| `⏸`  | yellow | Paused | Pane is in tmux copy/scroll mode — output is frozen      |

---

## Easy Setup using latest version

- latest binary is `latest/tc-watcher`
- easiest way to get started is to download the binary and then run
``` bash
cp target/release/tc-watcher ~/.local/bin/

tc-watcher # should work now!
```


---

## Requirements

- [Rust](https://www.rust-lang.org/tools/install) 1.85 or later (edition 2024)
- [tmux](https://github.com/tmux/tmux) installed; **tc-watcher must be launched from inside a tmux session**
- A terminal emulator with Unicode and 256-color support

---

## Installation

```bash
git clone https://github.com/jayfeng20/tmux-claude-watcher.git
cd tmux-claude-watcher
cargo build --release
```

The compiled binary will be at `target/release/tc-watcher`.

Optionally, copy it somewhere on your `$PATH`:

```bash
cp target/release/tc-watcher ~/.local/bin/
```

---

## Usage

The recommended setup is one tmux session for all your work, with `tc-watcher` running in a dedicated window so it stays visible without occupying a pane in your workspace:

```bash
# Start your session (if you don't have one already)
tmux new-session -s work

# From inside the session, open a watcher window
tmux new-window -n watcher
tc-watcher
```

Switch between windows with `Ctrl-b <number>` or `Ctrl-b w` to pick from a list. The watcher polls in the background regardless of which window you are viewing.

To script the whole setup:

```bash
tmux new-session -d -s work
tmux new-window -t work -n watcher
tmux send-keys -t work:watcher 'tc-watcher' Enter
tmux attach -t work
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

| Key         | Action                       |
|-------------|------------------------------|
| `↵ Enter`   | Jump to selected pane        |
| `q` / `Q`   | Quit                         |
| `j` / `↓`   | Move selection down          |
| `k` / `↑`   | Move selection up            |
| `?`         | Toggle help panel            |
| `Esc`       | Close help panel             |

**Jump to pane** runs `tmux switch-client` to bring the selected pane into focus. If the pane no longer exists a red error banner appears at the bottom of the monitor for 5 seconds.

**Returning to the monitor** after jumping: use `prefix + L` (last window), `prefix + <number>` (window by index), or `prefix + p`/`n` (previous/next window).

### Logs

Structured logs are written to `/tmp/tmux-claude-watcher.YYYY-MM-DD` (one file per day). To stream them while the monitor is running:

```bash
tail -f /tmp/tmux-claude-watcher.$(date +%Y-%m-%d)
```

To control log verbosity, set `RUST_LOG` before launching:

```bash
RUST_LOG=debug tmux-claude-watcher
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

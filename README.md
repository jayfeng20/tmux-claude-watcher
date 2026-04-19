# tc-watcher

A terminal UI that gives you a live, color-coded overview of every active tmux pane — what process is running, what it is doing, and how long it has been in that state.

Built with special awareness of [Claude Code](https://claude.ai/code): panes running `claude` are classified into fine-grained states (Thinking, Executing, Awaiting Input, Done) so you can glance at the monitor and know exactly where each agent is in its work cycle.

![Panel](examples/ui.png)
![Panel](examples/help.png)

---

## Download

Pre-built binaries are published automatically on every merge to `main`.

| Platform | Download |
|----------|----------|
| macOS (Apple Silicon) | [tc-watcher-aarch64-apple-darwin](https://github.com/jayfeng20/tmux-claude-watcher/releases/latest/download/tc-watcher-aarch64-apple-darwin) |
| macOS (Intel) | [tc-watcher-x86_64-apple-darwin](https://github.com/jayfeng20/tmux-claude-watcher/releases/latest/download/tc-watcher-x86_64-apple-darwin) |

```bash
# Apple Silicon
curl -L https://github.com/jayfeng20/tmux-claude-watcher/releases/latest/download/tc-watcher-aarch64-apple-darwin \
  -o tc-watcher && chmod +x tc-watcher && mv tc-watcher ~/.local/bin/

# Intel
curl -L https://github.com/jayfeng20/tmux-claude-watcher/releases/latest/download/tc-watcher-x86_64-apple-darwin \
  -o tc-watcher && chmod +x tc-watcher && mv tc-watcher ~/.local/bin/
```

Then launch it from inside a tmux session:

```bash
tc-watcher
```

> tc-watcher must be run from inside a tmux session. If you don't have one: `tmux new-session -s work`

---

## More

<details>
<summary>Features</summary>

- **Live polling** — refreshes all panes every 2 seconds without blocking the UI
- **State classification** — distinguishes shell, Claude, and tc-watcher panes and their sub-states
- **Timing column** — shows how long each pane has been in its current state
- **Active column** — indicates panes that are truly receiving keyboard input
- **Jump to pane** — press `↵` on any row to switch your terminal directly to that pane
- **Non-intrusive logging** — writes to a rolling daily file in `/tmp`

</details>

<details>
<summary>Key bindings</summary>

| Key | Action |
|-----|--------|
| `↵ Enter` | Jump to selected pane |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `q` / `Q` | Quit |
| `?` | Toggle help panel |
| `Esc` | Close help panel |

After jumping to a pane, return to the monitor with `prefix + L` (last window), `prefix + <number>`, or `prefix + p`/`n`.

</details>

<details>
<summary>Pane states</summary>

**Claude panes**

| Icon | Color | State | Meaning |
|------|-------|-------|---------|
| `◌` | orange | Thinking | Extended reasoning in progress |
| `◑` | yellow | Executing | Generating a response or running a tool |
| `❯` | red | Awaiting Input | Input box visible, Claude is asking a question |
| `!` | red | Awaiting Permission | Tool permission prompt needs approval |
| `✓` | green | Done | Task completed, input box visible, no question |
| `?` | dim | Unknown | State could not be determined |

**Shell panes** (`bash`, `zsh`, `fish`, `sh`)

| Icon | Color | State | Meaning |
|------|-------|-------|---------|
| `○` | green | Idle | Shell prompt visible — ready |
| `❯` | red | Awaiting Input | Process running or requesting input |
| `✗` | red | Error | Error output on the last line |

**tc-watcher panes**

| Icon | Color | State | Meaning |
|------|-------|-------|---------|
| `○` | green | Active | Monitor is running and polling |
| `⏸` | yellow | Paused | Pane is in tmux copy/scroll mode |

</details>

<details>
<summary>Requirements</summary>

- [tmux](https://github.com/tmux/tmux) installed; tc-watcher must be launched from inside a tmux session
- A terminal emulator with Unicode and 256-color support

</details>

<details>
<summary>Building from source</summary>

Requires [Rust](https://www.rust-lang.org/tools/install) 1.85 or later (edition 2024).

```bash
git clone https://github.com/jayfeng20/tmux-claude-watcher.git
cd tmux-monitor
cargo build --release
cp target/release/tc-watcher ~/.local/bin/
```

To run tests:

```bash
cargo test
```

</details>

<details>
<summary>Logs</summary>

Structured logs are written to `/tmp/tmux-claude-watcher.YYYY-MM-DD`. To stream them live:

```bash
tail -f /tmp/tmux-claude-watcher.$(date +%Y-%m-%d)
```

To increase verbosity:

```bash
RUST_LOG=debug tc-watcher
```

</details>

---

## License

[MIT](LICENSE)

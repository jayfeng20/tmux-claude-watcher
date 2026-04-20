# tc-watcher

A terminal UI that gives you a live, color-coded overview of every active tmux pane — what process is running, what it is doing, and how long it has been in that state.

Built with special awareness of [Claude Code](https://claude.ai/code): panes running `claude` are classified into fine-grained states (Thinking, Executing, Awaiting Input, Done) so you can glance at the monitor and know exactly where each agent is in its work cycle.

**Recommended workflow**

Run tc-watcher in its own dedicated session so it never shares window space with your work.

```bash
# Create a session just for the monitor
tmux new-session -s monitor
tc-watcher

# In another terminal, create your work session(s) as usual
tmux new-session -s work
```

- Navigate with `j`/`k` or `↑`/`↓` and press `↵` to jump to any pane across any session
- Press `prefix + R` from anywhere to jump straight back to the monitor
- Press `n` to create a new session, window, or pane; press `d` to delete the selected pane

By default tc-watcher binds `prefix + R` on startup and removes it on exit. Pass `--return-key <letter>` to use a different key.

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
- **Return key** — registers `prefix + R` (configurable) so you can jump back to the monitor from anywhere
- **Create sessions / windows / panes** — press `n` to open a drill-down picker; select an existing session to add a window, or choose `+ New` at any level
- **Delete panes** — press `d` on the selected row and confirm with `↵`
- **Priority sorting** — Claude panes awaiting input or permission float to the top so they never get buried
- **Non-intrusive logging** — writes to a rolling daily file in `/tmp`

</details>

<details>
<summary>Key bindings</summary>

| Key | Action |
|-----|--------|
| `↵ Enter` | Jump to selected pane |
| `j` / `↓` | Move selection down |
| `k` / `↑` | Move selection up |
| `n` | Open session/window/pane creator |
| `d` | Delete selected pane (asks for confirmation) |
| `q` / `Q` | Quit |
| `?` | Toggle help panel |
| `Esc` | Close overlay / cancel |

To return to the monitor after jumping to another pane, press `prefix + R` (or whichever key was configured with `--return-key`).

</details>

<details>
<summary>CLI flags</summary>

| Flag | Default | Description |
|------|---------|-------------|
| `-r`, `--return-key <KEY>` | `R` | tmux prefix key bound to jump back to the monitor. |

Example — use `prefix + M` instead of the default:

```bash
tc-watcher --return-key M
```

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

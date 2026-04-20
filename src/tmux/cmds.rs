//! Thin wrappers over every `tmux` subcommand used by this crate.
//!
//! Each function maps one-to-one to a tmux subcommand and returns the raw
//! `io::Result` without interpreting exit codes or stdout — that is the
//! caller's job. All `Command::new("tmux")` calls live here so no other
//! module needs to know the exact flags.

use super::pane::{PaneId, SessionName, WindowName};
use std::io;
use std::process::{Command, ExitStatus, Output};

// ---------------------------------------------------------------------------
// Query commands (return stdout)
// ---------------------------------------------------------------------------

/// `tmux list-panes -a -F <format>`
pub(crate) fn list_panes(format: &str) -> io::Result<Output> {
    run(&["list-panes", "-a", "-F", format])
}

/// `tmux capture-pane -p -t <target>`
pub(crate) fn capture_pane(target: &str) -> io::Result<Output> {
    run(&["capture-pane", "-p", "-t", target])
}

/// `tmux display-message -p "#{pane_id}"` — resolves this pane's stable `%N` id.
pub(crate) fn display_pane_id() -> io::Result<Output> {
    run(&["display-message", "-p", "#{pane_id}"])
}

/// `tmux list-keys -T prefix <key>` — empty stdout means the key is unbound.
pub(crate) fn list_prefix_key(key: char) -> io::Result<Output> {
    run(&["list-keys", "-T", "prefix", &key.to_string()])
}

// ---------------------------------------------------------------------------
// Mutation commands (return exit status)
// ---------------------------------------------------------------------------

/// `tmux switch-client -t %N`
pub(crate) fn switch_client(id: &PaneId) -> io::Result<ExitStatus> {
    run_status(&["switch-client", "-t", &id.target()])
}

/// `tmux new-session -d -s <name>`
pub(crate) fn new_session(name: &SessionName) -> io::Result<ExitStatus> {
    run_status(&["new-session", "-d", "-s", name.as_ref()])
}

/// `tmux new-window -t <session> -n <name>`
pub(crate) fn new_window(session: &SessionName, name: &WindowName) -> io::Result<ExitStatus> {
    run_status(&["new-window", "-t", session.as_ref(), "-n", name.as_ref()])
}

/// `tmux split-window -t <session:window>` — constructs the target internally.
pub(crate) fn split_window(session: &SessionName, window: &WindowName) -> io::Result<ExitStatus> {
    let target = format!("{}:{}", session.as_ref(), window.as_ref());
    run_status(&["split-window", "-t", &target])
}

/// `tmux kill-pane -t %N`
pub(crate) fn kill_pane(id: &PaneId) -> io::Result<ExitStatus> {
    run_status(&["kill-pane", "-t", &id.target()])
}

/// `tmux bind-key <key> switch-client -t <pane_id>`
pub(crate) fn bind_key(key: char, pane_id: &str) -> io::Result<ExitStatus> {
    run_status(&["bind-key", &key.to_string(), "switch-client", "-t", pane_id])
}

/// `tmux unbind-key <key>`
pub(crate) fn unbind_key(key: char) -> io::Result<ExitStatus> {
    run_status(&["unbind-key", &key.to_string()])
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

fn run(args: &[&str]) -> io::Result<Output> {
    Command::new("tmux").args(args).output()
}

fn run_status(args: &[&str]) -> io::Result<ExitStatus> {
    Command::new("tmux").args(args).status()
}

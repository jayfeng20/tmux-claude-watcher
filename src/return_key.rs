//! Return-key feature: lets the user jump back to the watcher from any tmux
//! session or window via `prefix + <key>` (default `R`).
//!
//! # Types
//! - [`Args`]    — CLI arguments parsed with `clap::Parser`.
//! - [`Binding`] — tmux binding lifecycle: registers on construction,
//!                 call [`Binding::deregister`] on exit.

use crate::tmux::cmds;
use clap::Parser;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

// ---------------------------------------------------------------------------
// CLI args
// ---------------------------------------------------------------------------

#[derive(Parser, Debug)]
#[command(about = "Monitor tmux panes and Claude agent activity")]
pub struct Args {
    /// Single tmux prefix key to bind for returning to the watcher from any
    /// session or window. Pass a different letter to avoid conflicts.
    #[arg(short = 'r', long, default_value = "R")]
    pub return_key: char,
}

// ---------------------------------------------------------------------------
// Binding — tmux registration lifecycle
// ---------------------------------------------------------------------------

/// A successfully registered tmux key binding.
/// Call [`deregister`] on exit to remove it cleanly.
///
/// [`deregister`]: Binding::deregister
pub struct Binding {
    pub key: Option<char>,
}

impl Binding {
    /// Registers `prefix + key` pointing at this pane.
    ///
    /// Returns `Err` if the pane ID cannot be resolved or the `bind-key`
    /// command fails. Returns `Ok(Binding { key: None })` when the key is
    /// already occupied — this is only a warning, not a hard failure, since the
    /// user may have intentionally bound it.
    pub fn register(key: char) -> Result<Self, BoxError> {
        let pane_id = resolve_pane_id()
            .ok_or("could not resolve tmux pane id — is tc-watcher running inside tmux?")?;

        if is_key_bound(key) {
            tracing::warn!(%key, "prefix+{key} already bound — return key skipped");
            return Ok(Self { key: None });
        }

        bind_key(key, &pane_id)?;

        tracing::info!(%key, "registered prefix+{key} → return to watcher");
        Ok(Self { key: Some(key) })
    }

    /// Removes the binding from tmux. No-op if `key` is `None`.
    pub fn deregister(self) {
        if let Some(key) = self.key {
            let _ = cmds::unbind_key(key);
            tracing::info!(%key, "deregistered prefix+{key}");
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Returns this pane's stable `%N` identifier via `#{pane_id}`.
fn resolve_pane_id() -> Option<String> {
    cmds::display_pane_id()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_owned())
}

/// Returns `true` if `prefix + key` is already bound in tmux.
fn is_key_bound(key: char) -> bool {
    cmds::list_prefix_key(key)
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Runs `tmux bind-key <key> switch-client -t <pane_id>`.
fn bind_key(key: char, pane_id: &str) -> Result<(), BoxError> {
    let status = cmds::bind_key(key, pane_id)?;
    if !status.success() {
        return Err(format!("tmux bind-key {key} failed (exit {status})").into());
    }
    Ok(())
}

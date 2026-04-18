//! One-shot tmux commands — the write/command side of CQS.
//!
//! [`crate::tmux::pane_manager::PaneManager`] covers the read side: polling
//! and state classification. This module covers the write side: actions
//! triggered by user input that change tmux state. Future commands such as
//! `kill-pane` or `send-keys` belong here too.

use crate::tmux::pane::PaneId;
use std::error::Error;
use std::process::Command;

/// Switches the active tmux client to the given pane.
///
/// Runs `tmux switch-client -t <target>`. Fails if there is no attached tmux
/// client (e.g. the monitor was started outside a tmux session) or if the
/// target pane no longer exists.
pub fn jump_to_pane(id: &PaneId) -> Result<(), Box<dyn Error + Send + Sync>> {
    let status = Command::new("tmux")
        .args(["switch-client", "-t", &id.target()])
        .status()?;
    if !status.success() {
        return Err(format!(
            "tmux switch-client failed for target {} — does it still exist?",
            id.target()
        )
        .into());
    }
    Ok(())
}

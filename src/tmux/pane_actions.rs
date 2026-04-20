//! One-shot tmux commands — the write/command side of CQS.
//!
//! [`crate::tmux::pane_manager::PaneManager`] covers the read side: polling
//! and state classification. This module covers the write side: actions
//! triggered by user input that change tmux state.

use crate::tmux::cmds;
use crate::tmux::pane::{PaneId, SessionName, WindowName};
use std::error::Error;

type BoxError = Box<dyn Error + Send + Sync>;

/// Switches the active tmux client to the given pane.
pub fn jump_to_pane(id: &PaneId) -> Result<(), BoxError> {
    let status = cmds::switch_client(id)?;
    if !status.success() {
        return Err(format!(
            "tmux switch-client failed for target {} — does it still exist?",
            id.target()
        )
        .into());
    }
    Ok(())
}

/// Creates a new detached session with the given name.
pub fn new_session(name: &SessionName) -> Result<(), BoxError> {
    let status = cmds::new_session(name)?;
    if !status.success() {
        return Err(format!("tmux new-session failed for session {name}").into());
    }
    Ok(())
}

/// Creates a new window in the given session with the given name.
pub fn new_window(session: &SessionName, name: &WindowName) -> Result<(), BoxError> {
    let status = cmds::new_window(session, name)?;
    if !status.success() {
        return Err(format!("tmux new-window failed for session {session}").into());
    }
    Ok(())
}

/// Splits the given window to create a new pane.
pub fn split_pane(session: &SessionName, window: &WindowName) -> Result<(), BoxError> {
    let status = cmds::split_window(session, window)?;
    if !status.success() {
        return Err(format!("tmux split-window failed for {session}:{window}").into());
    }
    Ok(())
}

/// Kills the given pane.
pub fn kill_pane(id: &PaneId) -> Result<(), BoxError> {
    let status = cmds::kill_pane(id)?;
    if !status.success() {
        return Err(format!(
            "tmux kill-pane failed for target {} — does it still exist?",
            id.target()
        )
        .into());
    }
    Ok(())
}

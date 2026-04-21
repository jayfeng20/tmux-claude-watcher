//! Tmux pane state management.
//!
//! [`PaneManager`] is the single point of contact with the tmux process.
//! It owns all tmux I/O (`list-panes`, `capture-pane`), parses the output,
//! classifies each pane's state, and diffs successive snapshots to maintain
//! accurate timing metadata.

use crate::tmux::cmds;
use crate::tmux::pane::{PaneId, PaneInfo, PaneState, ProcessOutcome, ShellStatus};
use std::collections::HashMap;
use std::error::Error;
use std::sync::Arc;
use std::time::SystemTime;
use strum::{AsRefStr, EnumIter, EnumString, IntoEnumIterator};

/// Tmux format variables used to query pane information.
/// Declaration order determines the `list-panes` output format — do not reorder.
#[derive(Debug, PartialEq, Eq, Hash, AsRefStr, EnumIter, EnumString)]
#[strum(serialize_all = "snake_case")]
enum TmuxVar {
    PaneId,
    PaneActive,
    PaneCurrentCommand,
    PaneInMode,
    WindowIndex,
    WindowName,
    WindowActive,
    SessionName,
    SessionAttached,
    PaneLastExitStatus,
}

/// Parsed tmux metadata for one pane, before classification.
/// Private to this module — consumers always receive fully-classified [`PaneInfo`].
struct RawPane {
    id: PaneId,
    pane_active: bool,
    window_active: bool,
    /// Number of terminal clients attached to this pane's session; 0 means no one is viewing it.
    session_attached: bool,
    pane_in_mode: bool,
    current_cmd: String,
    last_exit_status: i32,
}

/// Manages the state of all active tmux panes.
#[derive(Debug)]
pub struct PaneManager {
    /// Shared snapshot of the latest pane list. Wrapped in [`Arc`] so the
    /// poller can hand it to the UI in O(1) without copying the data.
    active_panes: Arc<Vec<PaneInfo>>,
    /// Ordered list of tmux format variables; order must match the `list_panes` format string.
    tmux_variables: Vec<TmuxVar>,
}

impl Default for PaneManager {
    fn default() -> Self {
        Self::new()
    }
}

impl PaneManager {
    pub fn new() -> Self {
        PaneManager {
            active_panes: Arc::new(Vec::new()),
            tmux_variables: TmuxVar::iter().collect(),
        }
    }

    /// Returns a shared reference to the current pane snapshot.
    /// Cloning the returned [`Arc`] is O(1) — no pane data is copied.
    pub fn panes(&self) -> Arc<Vec<PaneInfo>> {
        Arc::clone(&self.active_panes)
    }

    /// Queries tmux, classifies each pane, diffs against the previous snapshot,
    /// and updates `active_panes` in place.
    ///
    /// # Errors
    /// Returns an error if `tmux list-panes` fails or its output cannot be parsed.
    /// Individual `capture-pane` failures are logged and treated as empty content
    /// rather than propagated.
    #[tracing::instrument(skip(self))]
    pub fn refresh(&mut self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let raw_panes = self.list_panes()?;

        let current_snapshot = raw_panes
            .into_iter()
            .map(|raw| {
                let content = Self::capture_pane(&raw.id.target()).unwrap_or_else(|e| {
                    tracing::warn!(target = %raw.id.target(), error = %e, "capture-pane failed");
                    String::new()
                });
                let state = PaneState::from_process(&raw.current_cmd, &content, raw.pane_in_mode);
                PaneInfo {
                    state,
                    id: raw.id,
                    pane_active: raw.pane_active,
                    window_active: raw.window_active,
                    session_attached: raw.session_attached,
                    pane_in_mode: raw.pane_in_mode,
                    current_cmd: raw.current_cmd,
                    last_exit_status: raw.last_exit_status,
                    last_updated: SystemTime::now(),
                    last_focused_at: None,
                    status_changed_at: None,
                }
            })
            .collect();

        self.derive_latest_snapshot(current_snapshot);
        tracing::info!(count = self.active_panes.len(), "panes refreshed");
        Ok(())
    }

    /// Runs `tmux list-panes -a` and returns raw parsed metadata for each pane.
    #[tracing::instrument(skip(self))]
    fn list_panes(&self) -> Result<Vec<RawPane>, Box<dyn Error + Send + Sync>> {
        let output_fmt = self
            .tmux_variables
            .iter()
            .map(|v| format!("#{{{}}}", v.as_ref()))
            .collect::<Vec<String>>()
            .join("|");

        let output = cmds::list_panes(&output_fmt)?;
        let stdout = String::from_utf8(output.stdout)?;
        stdout.lines().map(|l| self.parse_pane_info(l)).collect()
    }

    /// Runs `tmux capture-pane -p -t <target>` and returns the visible pane content.
    #[tracing::instrument]
    fn capture_pane(target: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let output = cmds::capture_pane(target)?;
        Ok(String::from_utf8(output.stdout)?)
    }

    /// Parses a `|`-delimited tmux format line into a [`RawPane`].
    #[tracing::instrument(skip(self, pane_info))]
    fn parse_pane_info(&self, pane_info: &str) -> Result<RawPane, Box<dyn Error + Send + Sync>> {
        let mut tmux_vars = self.tmux_variables.iter();

        let info_map: HashMap<&str, &str> =
            pane_info.split('|').fold(HashMap::new(), |mut acc, v| {
                if let Some(var) = tmux_vars.next() {
                    acc.insert(var.as_ref(), v);
                }
                acc
            });

        let get = |key: &TmuxVar| -> Result<&str, Box<dyn Error + Send + Sync>> {
            info_map
                .get(key.as_ref())
                .copied()
                .ok_or_else(|| format!("missing tmux field: {}", key.as_ref()).into())
        };

        // #{pane_id} is returned as "%N" by tmux; strip the leading '%' before parsing.
        let pane_id_num = get(&TmuxVar::PaneId)?
            .trim_start_matches('%')
            .parse::<u32>()?;

        Ok(RawPane {
            id: PaneId {
                session_name: get(&TmuxVar::SessionName)?.to_string(),
                window_index: get(&TmuxVar::WindowIndex)?.parse::<u32>()?,
                window_name: get(&TmuxVar::WindowName)?.to_string(),
                pane_id: pane_id_num,
            },
            pane_active: get(&TmuxVar::PaneActive)?.parse::<u32>()? != 0,
            window_active: get(&TmuxVar::WindowActive)?.parse::<u32>()? != 0,
            session_attached: get(&TmuxVar::SessionAttached)?.parse::<u32>().unwrap_or(0) != 0,
            pane_in_mode: get(&TmuxVar::PaneInMode)?.parse::<u32>()? != 0,
            current_cmd: get(&TmuxVar::PaneCurrentCommand)?.to_string(),
            last_exit_status: get(&TmuxVar::PaneLastExitStatus)?
                .parse::<i32>()
                .unwrap_or(0),
        })
    }

    /// Builds the next `active_panes` snapshot by merging freshly-polled pane data
    /// with the previous snapshot with some derived fields, e.g. state
    #[tracing::instrument(skip(self, current_snapshot), fields(count = current_snapshot.len()))]
    fn derive_latest_snapshot(&mut self, mut current_snapshot: Vec<PaneInfo>) {
        let prev_snapshot: HashMap<(String, u32), &PaneInfo> = self
            .active_panes
            .iter()
            .map(|p| ((p.id.session_name.clone(), p.id.pane_id), p))
            .collect();

        for current in &mut current_snapshot {
            let key = (current.id.session_name.clone(), current.id.pane_id);
            match prev_snapshot.get(&key) {
                Some(prev) => {
                    update_pane_state(current, prev);
                    update_status_changed_at(current, prev);
                    update_last_focused_at(current, prev);
                }
                None => {
                    tracing::debug!(pane_id = current.id.pane_id, cmd = %current.current_cmd, "new pane discovered");
                    current.status_changed_at = Some(SystemTime::now());
                    current.last_focused_at = current.pane_active.then(SystemTime::now);
                }
            }
        }

        sort_panes(&mut current_snapshot);
        self.active_panes = Arc::new(current_snapshot);
    }
}

/// Sets `current_pane.state` for transitions requiring a diff against the previous poll:
/// marks `JustFinished` when a subprocess exits, and preserves it until the user focuses the pane.
fn update_pane_state(current: &mut PaneInfo, prev: &PaneInfo) {
    let just_focused = current.pane_active && !prev.pane_active;

    match &prev.state {
        // Other(cmd) → Shell: subprocess exited. Notify unless user is already watching.
        PaneState::Other(cmd) if !current.pane_active => {
            if let PaneState::Shell(kind, _) = &current.state {
                current.state = PaneState::Shell(
                    kind.clone(),
                    ShellStatus::JustFinished {
                        cmd: cmd.clone(),
                        outcome: ProcessOutcome::from_exit_status(current.last_exit_status),
                    },
                );
            }
        }
        // Preserve JustFinished across polls until the user focuses the pane.
        PaneState::Shell(kind, ShellStatus::JustFinished { cmd, outcome })
            if !just_focused && matches!(current.state, PaneState::Shell(_, ShellStatus::Idle)) =>
        {
            current.state = PaneState::Shell(
                kind.clone(),
                ShellStatus::JustFinished {
                    cmd: cmd.clone(),
                    outcome: outcome.clone(),
                },
            );
        }
        _ => {}
    }
}

fn update_status_changed_at(current: &mut PaneInfo, prev: &PaneInfo) {
    current.status_changed_at = if prev.state != current.state {
        tracing::debug!(pane_id = current.id.pane_id, old = ?prev.state, new = ?current.state, "state changed");
        Some(SystemTime::now())
    } else {
        prev.status_changed_at
    };
}

fn update_last_focused_at(current: &mut PaneInfo, prev: &PaneInfo) {
    current.last_focused_at = if current.pane_active && !prev.pane_active {
        tracing::debug!(pane_id = current.id.pane_id, "pane focused");
        Some(SystemTime::now())
    } else {
        prev.last_focused_at
    };
}

/// Sorts panes in place: urgency tier first, then most recent activity within each tier.
fn sort_panes(panes: &mut [PaneInfo]) {
    panes.sort_by(|pane_a, pane_b| {
        pane_a
            .state
            .urgency_tier()
            .cmp(&pane_b.state.urgency_tier())
            .then_with(|| {
                pane_b
                    .most_recent_activity()
                    .cmp(&pane_a.most_recent_activity())
            })
    });
}

#[cfg(test)]
#[path = "pane_manager_tests.rs"]
mod tests;

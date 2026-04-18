//! Tmux pane state management.
//!
//! [`PaneManager`] is the single point of contact with the tmux process.
//! It owns all tmux I/O (`list-panes`, `capture-pane`), parses the output,
//! classifies each pane's state, and diffs successive snapshots to maintain
//! accurate timing metadata.

use crate::tmux::pane::{PaneId, PaneInfo, PaneState};
use std::collections::HashMap;
use std::error::Error;
use std::process::Command;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
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
    PaneLastActive,
    WindowIndex,
    WindowName,
    WindowActive,
    SessionName,
    SessionAttached,
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
    /// Unix timestamp (seconds) from `#{pane_last_active}`; 0 means never active.
    pane_last_active_secs: u64,
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

        let fresh = raw_panes
            .into_iter()
            .map(|raw| {
                let content = Self::capture_pane(&raw.id.target()).unwrap_or_else(|e| {
                    tracing::warn!(target = %raw.id.target(), error = %e, "capture-pane failed");
                    String::new()
                });
                let state = PaneState::from_process(&raw.current_cmd, &content);
                let last_focused_at = match raw.pane_last_active_secs {
                    0 => None,
                    secs => Some(UNIX_EPOCH + Duration::from_secs(secs)),
                };
                PaneInfo {
                    state,
                    id: raw.id,
                    pane_active: raw.pane_active,
                    window_active: raw.window_active,
                    session_attached: raw.session_attached,
                    pane_in_mode: raw.pane_in_mode,
                    current_cmd: raw.current_cmd,
                    last_updated: SystemTime::now(),
                    last_focused_at,
                    status_changed_at: None,
                }
            })
            .collect();

        self.merge_panes(fresh);
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

        let output = Command::new("tmux")
            .args(["list-panes", "-a", "-F", &output_fmt])
            .output()?;

        let stdout = String::from_utf8(output.stdout)?;
        stdout.lines().map(|l| self.parse_pane_info(l)).collect()
    }

    /// Runs `tmux capture-pane -p -t <target>` and returns the visible pane content.
    #[tracing::instrument]
    fn capture_pane(target: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let output = Command::new("tmux")
            .args(["capture-pane", "-p", "-t", target])
            .output()?;
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
            pane_last_active_secs: get(&TmuxVar::PaneLastActive)?.parse::<u64>().unwrap_or(0),
        })
    }

    /// Diffs `fresh` against `active_panes` and carries forward timing fields.
    ///
    /// This is the sole authority on `status_changed_at` and `last_focused_at` —
    /// both are initialized to [`TIMING_PENDING`] in `refresh` and always set here.
    #[tracing::instrument(skip(self, fresh), fields(count = fresh.len()))]
    fn merge_panes(&mut self, mut fresh: Vec<PaneInfo>) {
        let prev: HashMap<(String, u32), &PaneInfo> = self
            .active_panes
            .iter()
            .map(|p| ((p.id.session_name.clone(), p.id.pane_id), p))
            .collect();

        for pane in &mut fresh {
            let key = (pane.id.session_name.clone(), pane.id.pane_id);
            match prev.get(&key) {
                Some(p) => {
                    pane.status_changed_at = if p.state != pane.state {
                        tracing::debug!(pane_id = pane.id.pane_id, old = ?p.state, new = ?pane.state, "state changed");
                        Some(SystemTime::now())
                    } else {
                        p.status_changed_at
                    };
                    // last_focused_at comes directly from #{pane_last_active} — no
                    // transition inference needed, so carry the fresh value through.
                }
                // New pane — start its timers from now.
                None => {
                    tracing::debug!(pane_id = pane.id.pane_id, cmd = %pane.current_cmd, "new pane discovered");
                    pane.status_changed_at = Some(SystemTime::now());
                    // last_focused_at already set from #{pane_last_active} in refresh()
                }
            }
        }

        self.active_panes = Arc::new(fresh);
    }
}

#[cfg(test)]
#[path = "pane_manager_tests.rs"]
mod tests;

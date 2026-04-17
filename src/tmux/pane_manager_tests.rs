use super::*;
use crate::tmux::pane::{PaneId, PaneInfo, PaneState, ShellKind, ShellStatus};
use std::time::{Duration, SystemTime};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_manager() -> PaneManager {
    PaneManager::new()
}

/// Builds a pipe-delimited line in `TmuxVar::iter()` order.
fn make_pane_line(
    pane_id: &str,
    pane_active: &str,
    pane_current_command: &str,
    pane_in_mode: &str,
    pane_last_active: &str,
    window_index: &str,
    window_name: &str,
    window_active: &str,
    session_name: &str,
) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        pane_id,
        pane_active,
        pane_current_command,
        pane_in_mode,
        pane_last_active,
        window_index,
        window_name,
        window_active,
        session_name
    )
}

/// Constructs a minimal [`PaneInfo`] for use in `merge_panes` tests.
fn make_pane_info(
    session: &str,
    pane_id: u32,
    pane_active: bool,
    state: PaneState,
    status_changed_at: Option<SystemTime>,
    last_focused_at: Option<SystemTime>,
) -> PaneInfo {
    PaneInfo {
        id: PaneId {
            session_name: session.to_string(),
            window_index: 0,
            window_name: "win".to_string(),
            pane_id,
        },
        pane_active,
        window_active: true,
        pane_in_mode: false,
        current_cmd: "bash".to_string(),
        state,
        last_updated: SystemTime::now(),
        status_changed_at,
        last_focused_at,
    }
}

// ---------------------------------------------------------------------------
// parse_pane_info — parses a tmux format line into a RawPane
// ---------------------------------------------------------------------------

#[test]
fn parse_raw_pane_valid() {
    let mgr = make_manager();
    let line = make_pane_line(
        "%3",
        "1",
        "bash",
        "0",
        "1700000000",
        "2",
        "editor",
        "1",
        "work",
    );
    let raw = mgr
        .parse_pane_info(&line)
        .expect("should parse successfully");

    assert_eq!(raw.id.pane_id, 3);
    assert_eq!(raw.id.session_name, "work");
    assert_eq!(raw.id.window_index, 2);
    assert_eq!(raw.id.window_name, "editor");
    assert!(raw.pane_active);
    assert!(raw.window_active);
    assert!(!raw.pane_in_mode);
    assert_eq!(raw.current_cmd, "bash");
    assert_eq!(raw.pane_last_active_secs, 1700000000);
}

#[test]
fn parse_raw_pane_inactive() {
    let mgr = make_manager();
    let line = make_pane_line("%0", "0", "zsh", "0", "0", "0", "term", "1", "main");
    let raw = mgr.parse_pane_info(&line).expect("should parse");
    assert!(!raw.pane_active);
    assert_eq!(raw.id.pane_id, 0);
}

#[test]
fn parse_raw_pane_copy_mode() {
    let mgr = make_manager();
    let line = make_pane_line("%1", "1", "bash", "1", "0", "0", "logs", "1", "sys");
    let raw = mgr.parse_pane_info(&line).expect("should parse");
    assert!(raw.pane_in_mode);
}

/// tmux returns pane IDs as "%N"; bare numbers should also be accepted.
#[test]
fn parse_raw_pane_id_without_percent_prefix() {
    let mgr = make_manager();
    let line = make_pane_line("5", "1", "fish", "0", "0", "1", "term", "1", "dev");
    let raw = mgr
        .parse_pane_info(&line)
        .expect("bare pane_id should work");
    assert_eq!(raw.id.pane_id, 5);
}

#[test]
fn parse_raw_pane_non_numeric_window_index_errors() {
    let mgr = make_manager();
    let line = make_pane_line(
        "%0",
        "1",
        "bash",
        "0",
        "0",
        "notanumber",
        "win",
        "1",
        "sess",
    );
    assert!(mgr.parse_pane_info(&line).is_err());
}

#[test]
fn parse_raw_pane_too_few_fields_errors() {
    let mgr = make_manager();
    let line = "%0|1|bash|0|1"; // missing window_name and session_name
    assert!(mgr.parse_pane_info(line).is_err());
}

// ---------------------------------------------------------------------------
// merge_panes — diff logic and timing field management
// ---------------------------------------------------------------------------

#[test]
fn merge_panes_new_pane_initialises_timers() {
    let mut mgr = make_manager();
    let before = SystemTime::now();
    let fresh = vec![make_pane_info(
        "work",
        1,
        false,
        PaneState::Shell(ShellKind::Bash, ShellStatus::AwaitingInput),
        None,
        None,
    )];
    mgr.merge_panes(fresh);

    let pane = &mgr.active_panes[0];
    assert!(
        pane.status_changed_at >= Some(before),
        "status_changed_at should be set to now"
    );
    assert_eq!(
        pane.last_focused_at, None,
        "new pane has never been focused"
    );
}

#[test]
fn merge_panes_unchanged_state_carries_forward_status_changed_at() {
    let mut mgr = make_manager();
    let old_time = SystemTime::now() - Duration::from_secs(3600);
    let state = PaneState::Shell(ShellKind::Zsh, ShellStatus::AwaitingInput);

    // Seed active_panes with a known status_changed_at.
    mgr.active_panes = Arc::new(vec![make_pane_info(
        "work",
        1,
        false,
        state.clone(),
        Some(old_time),
        None,
    )]);

    // Fresh pane has the same state — status_changed_at must be carried forward.
    mgr.merge_panes(vec![make_pane_info("work", 1, false, state, None, None)]);

    assert_eq!(mgr.active_panes[0].status_changed_at, Some(old_time));
}

#[test]
fn merge_panes_changed_state_resets_status_changed_at() {
    let mut mgr = make_manager();
    let old_time = SystemTime::now() - Duration::from_secs(3600);

    mgr.active_panes = Arc::new(vec![make_pane_info(
        "work",
        1,
        false,
        PaneState::Shell(ShellKind::Bash, ShellStatus::AwaitingInput),
        Some(old_time),
        None,
    )]);

    let before = SystemTime::now();
    mgr.merge_panes(vec![make_pane_info(
        "work",
        1,
        false,
        PaneState::Shell(ShellKind::Bash, ShellStatus::Processing), // state changed
        None,
        None,
    )]);

    assert!(mgr.active_panes[0].status_changed_at >= Some(before));
    assert!(mgr.active_panes[0].status_changed_at > Some(old_time));
}

#[test]
fn parse_pane_last_active_nonzero_becomes_some_systemtime() {
    let mgr = make_manager();
    let line = make_pane_line(
        "%0",
        "1",
        "bash",
        "0",
        "1700000000",
        "0",
        "term",
        "1",
        "main",
    );
    let raw = mgr.parse_pane_info(&line).expect("should parse");
    assert_eq!(raw.pane_last_active_secs, 1700000000);
}

#[test]
fn parse_pane_last_active_zero_stays_zero() {
    let mgr = make_manager();
    let line = make_pane_line("%0", "1", "bash", "0", "0", "0", "term", "1", "main");
    let raw = mgr.parse_pane_info(&line).expect("should parse");
    assert_eq!(raw.pane_last_active_secs, 0);
}

#[test]
fn merge_panes_last_focused_at_carried_from_fresh_pane() {
    let mut mgr = make_manager();
    let focus_time = SystemTime::now() - Duration::from_secs(3600);
    let state = PaneState::Shell(ShellKind::Bash, ShellStatus::AwaitingInput);

    mgr.active_panes = Arc::new(vec![make_pane_info(
        "work",
        1,
        true,
        state.clone(),
        Some(SystemTime::now()),
        None,
    )]);

    // Fresh pane carries a focus timestamp from #{pane_last_active} — merge must preserve it.
    mgr.merge_panes(vec![make_pane_info(
        "work",
        1,
        true,
        state,
        None,
        Some(focus_time),
    )]);

    assert_eq!(mgr.active_panes[0].last_focused_at, Some(focus_time));
}

// ---------------------------------------------------------------------------
// Format string integrity
// ---------------------------------------------------------------------------

#[test]
fn tmux_variables_order_is_stable_across_instances() {
    let mgr1 = make_manager();
    let vars1: Vec<&str> = mgr1.tmux_variables.iter().map(|v| v.as_ref()).collect();
    let mgr2 = make_manager();
    let vars2: Vec<&str> = mgr2.tmux_variables.iter().map(|v| v.as_ref()).collect();
    assert_eq!(vars1, vars2, "variable order must be deterministic");
}

#[test]
fn list_panes_format_contains_all_vars() {
    let mgr = make_manager();
    let fmt = mgr
        .tmux_variables
        .iter()
        .map(|v| format!("#{{{}}}", v.as_ref()))
        .collect::<Vec<_>>()
        .join("|");

    for var in TmuxVar::iter() {
        assert!(
            fmt.contains(&format!("#{{{}}}", var.as_ref())),
            "format string missing: {}",
            var.as_ref()
        );
    }
}

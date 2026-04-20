use super::*;
use crate::tmux::pane::{
    ClaudeStatus, PaneId, PaneInfo, PaneState, ShellKind, ShellStatus, TcWatcherStatus,
};
use std::time::{Duration, SystemTime};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_manager() -> PaneManager {
    PaneManager::new()
}

struct PaneLine<'a> {
    pane_id: &'a str,
    pane_active: &'a str,
    pane_current_command: &'a str,
    pane_in_mode: &'a str,
    window_index: &'a str,
    window_name: &'a str,
    window_active: &'a str,
    session_name: &'a str,
    session_attached: &'a str,
}

/// Builds a pipe-delimited line in `TmuxVar::iter()` order.
fn make_pane_line(p: PaneLine<'_>) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}|{}|{}|{}",
        p.pane_id,
        p.pane_active,
        p.pane_current_command,
        p.pane_in_mode,
        p.window_index,
        p.window_name,
        p.window_active,
        p.session_name,
        p.session_attached
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
        session_attached: true,
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
    let line = make_pane_line(PaneLine {
        pane_id: "%3",
        pane_active: "1",
        pane_current_command: "bash",
        pane_in_mode: "0",
        window_index: "2",
        window_name: "editor",
        window_active: "1",
        session_name: "work",
        session_attached: "1",
    });
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
}

#[test]
fn parse_raw_pane_inactive() {
    let mgr = make_manager();
    let line = make_pane_line(PaneLine {
        pane_id: "%0",
        pane_active: "0",
        pane_current_command: "zsh",
        pane_in_mode: "0",
        window_index: "0",
        window_name: "term",
        window_active: "1",
        session_name: "main",
        session_attached: "1",
    });
    let raw = mgr.parse_pane_info(&line).expect("should parse");
    assert!(!raw.pane_active);
    assert_eq!(raw.id.pane_id, 0);
}

#[test]
fn parse_raw_pane_copy_mode() {
    let mgr = make_manager();
    let line = make_pane_line(PaneLine {
        pane_id: "%1",
        pane_active: "1",
        pane_current_command: "bash",
        pane_in_mode: "1",
        window_index: "0",
        window_name: "logs",
        window_active: "1",
        session_name: "sys",
        session_attached: "1",
    });
    let raw = mgr.parse_pane_info(&line).expect("should parse");
    assert!(raw.pane_in_mode);
}

/// tmux returns pane IDs as "%N"; bare numbers should also be accepted.
#[test]
fn parse_raw_pane_id_without_percent_prefix() {
    let mgr = make_manager();
    let line = make_pane_line(PaneLine {
        pane_id: "5",
        pane_active: "1",
        pane_current_command: "fish",
        pane_in_mode: "0",
        window_index: "1",
        window_name: "term",
        window_active: "1",
        session_name: "dev",
        session_attached: "1",
    });
    let raw = mgr
        .parse_pane_info(&line)
        .expect("bare pane_id should work");
    assert_eq!(raw.id.pane_id, 5);
}

#[test]
fn parse_raw_pane_non_numeric_window_index_errors() {
    let mgr = make_manager();
    let line = make_pane_line(PaneLine {
        pane_id: "%0",
        pane_active: "1",
        pane_current_command: "bash",
        pane_in_mode: "0",
        window_index: "notanumber",
        window_name: "win",
        window_active: "1",
        session_name: "sess",
        session_attached: "1",
    });
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
        PaneState::Shell(ShellKind::Bash, ShellStatus::Idle), // state changed
        None,
        None,
    )]);

    assert!(mgr.active_panes[0].status_changed_at >= Some(before));
    assert!(mgr.active_panes[0].status_changed_at > Some(old_time));
}

#[test]
fn merge_panes_records_focus_when_pane_becomes_active() {
    let mut mgr = make_manager();
    let state = PaneState::Shell(ShellKind::Bash, ShellStatus::Idle);

    // Seed with an inactive pane that has never been focused.
    mgr.active_panes = Arc::new(vec![make_pane_info(
        "work",
        1,
        false,
        state.clone(),
        Some(SystemTime::now()),
        None,
    )]);

    let before = SystemTime::now();
    // Pane transitions to active — merge_panes must record the focus time.
    mgr.merge_panes(vec![make_pane_info("work", 1, true, state, None, None)]);

    assert!(
        mgr.active_panes[0].last_focused_at >= Some(before),
        "last_focused_at should be set when pane becomes active"
    );
}

#[test]
fn merge_panes_carries_forward_last_focused_at_while_still_active() {
    let mut mgr = make_manager();
    let focus_time = SystemTime::now() - Duration::from_secs(3600);
    let state = PaneState::Shell(ShellKind::Bash, ShellStatus::Idle);

    // Seed with an already-active pane that was focused an hour ago.
    mgr.active_panes = Arc::new(vec![make_pane_info(
        "work",
        1,
        true,
        state.clone(),
        Some(SystemTime::now()),
        Some(focus_time),
    )]);

    // Pane remains active — last_focused_at must not be reset.
    mgr.merge_panes(vec![make_pane_info("work", 1, true, state, None, None)]);

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

// ---------------------------------------------------------------------------
// sort_panes — urgency tier and recency ordering
// ---------------------------------------------------------------------------

#[test]
fn sort_panes_tier_beats_recency() {
    // A very recently-updated shell pane must still rank below an older
    // Claude pane that needs user action.
    let old = SystemTime::now() - Duration::from_secs(3600);
    let new = SystemTime::now();
    let mut panes = vec![
        make_pane_info(
            "s",
            1,
            false,
            PaneState::Shell(ShellKind::Bash, ShellStatus::Idle),
            Some(new),
            None,
        ),
        make_pane_info(
            "s",
            2,
            false,
            PaneState::Claude(ClaudeStatus::AwaitingInput),
            Some(old),
            None,
        ),
    ];
    sort_panes(&mut panes);
    assert!(
        matches!(
            panes[0].state,
            PaneState::Claude(ClaudeStatus::AwaitingInput)
        ),
        "awaiting-input Claude should be first regardless of recency"
    );
}

#[test]
fn sort_panes_tc_watcher_is_first_regardless_of_state() {
    let mut panes = vec![
        make_pane_info(
            "s",
            1,
            false,
            PaneState::Claude(ClaudeStatus::AwaitingInput),
            Some(SystemTime::now()),
            None,
        ),
        make_pane_info(
            "s",
            2,
            false,
            PaneState::TcWatcher(TcWatcherStatus::Active),
            Some(SystemTime::now()),
            None,
        ),
    ];
    sort_panes(&mut panes);
    assert!(matches!(panes[0].state, PaneState::TcWatcher(_)));
}

#[test]
fn sort_panes_within_same_tier_more_recent_comes_first() {
    let older = SystemTime::now() - Duration::from_secs(3600);
    let newer = SystemTime::now();
    let mut panes = vec![
        make_pane_info(
            "s",
            1,
            false,
            PaneState::Shell(ShellKind::Bash, ShellStatus::Idle),
            Some(older),
            None,
        ),
        make_pane_info(
            "s",
            2,
            false,
            PaneState::Shell(ShellKind::Zsh, ShellStatus::Idle),
            Some(newer),
            None,
        ),
    ];
    sort_panes(&mut panes);
    assert_eq!(
        panes[0].id.pane_id, 2,
        "the pane updated more recently should appear first"
    );
}

#[test]
fn merge_panes_new_active_pane_gets_last_focused_at() {
    // A pane that appears already-active in its first refresh should have
    // last_focused_at initialised to now, not left as None.
    let mut mgr = make_manager();
    let before = SystemTime::now();
    let pane = make_pane_info(
        "s",
        1,
        true, /* active */
        PaneState::Shell(ShellKind::Bash, ShellStatus::Idle),
        None,
        None,
    );
    mgr.merge_panes(vec![pane]);
    assert!(
        mgr.active_panes[0].last_focused_at >= Some(before),
        "an already-active new pane should have last_focused_at set immediately"
    );
}

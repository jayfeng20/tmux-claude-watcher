use super::{App, AppAction};
use crate::tmux::pane::{PaneId, PaneInfo, PaneState, ShellKind, ShellStatus};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use std::sync::Arc;
use std::time::SystemTime;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn panes(n: usize) -> Arc<Vec<PaneInfo>> {
    Arc::new(
        (0..n)
            .map(|i| PaneInfo {
                id: PaneId {
                    session_name: "work".into(),
                    window_index: 0,
                    window_name: "win".into(),
                    pane_id: i as u32,
                },
                pane_active: false,
                window_active: true,
                session_attached: true,
                pane_in_mode: false,
                current_cmd: "bash".into(),
                state: PaneState::Shell(ShellKind::Bash, ShellStatus::Idle),
                last_updated: SystemTime::now(),
                status_changed_at: None,
                last_focused_at: None,
            })
            .collect(),
    )
}

// ---------------------------------------------------------------------------
// Normal mode
// ---------------------------------------------------------------------------

#[test]
fn q_returns_quit() {
    let mut app = App::new();
    assert!(matches!(
        app.handle_key(key(KeyCode::Char('q'))),
        Some(AppAction::Quit)
    ));
}

#[test]
fn enter_without_panes_returns_none() {
    let mut app = App::new();
    assert!(app.handle_key(key(KeyCode::Enter)).is_none());
}

#[test]
fn enter_with_pane_returns_jump_to_pane() {
    let mut app = App::new();
    app.update_panes(panes(1));
    assert!(matches!(
        app.handle_key(key(KeyCode::Enter)),
        Some(AppAction::JumpToPane(_))
    ));
}

#[test]
fn d_without_panes_stays_in_normal_mode() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('d')));
    // still Normal — q should quit
    assert!(matches!(
        app.handle_key(key(KeyCode::Char('q'))),
        Some(AppAction::Quit)
    ));
}

#[test]
fn jk_navigation_changes_which_pane_enter_jumps_to() {
    let mut app = App::new();
    app.update_panes(panes(3));
    app.handle_key(key(KeyCode::Char('j')));
    app.handle_key(key(KeyCode::Char('j')));
    if let Some(AppAction::JumpToPane(id)) = app.handle_key(key(KeyCode::Enter)) {
        assert_eq!(id.pane_id, 2);
    } else {
        panic!("expected JumpToPane after two j presses");
    }
}

#[test]
fn k_at_top_wraps_to_last() {
    let mut app = App::new();
    app.update_panes(panes(3));
    app.handle_key(key(KeyCode::Char('k'))); // wrap: 0 → 2
    if let Some(AppAction::JumpToPane(id)) = app.handle_key(key(KeyCode::Enter)) {
        assert_eq!(id.pane_id, 2);
    } else {
        panic!("expected JumpToPane");
    }
}

// ---------------------------------------------------------------------------
// Help mode — keys are swallowed until ? or Esc
// ---------------------------------------------------------------------------

#[test]
fn question_mark_opens_help_and_swallows_q() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('?')));
    // q should be swallowed while Help is open
    assert!(app.handle_key(key(KeyCode::Char('q'))).is_none());
    // second ? closes help
    app.handle_key(key(KeyCode::Char('?')));
    // now back to Normal
    assert!(matches!(
        app.handle_key(key(KeyCode::Char('q'))),
        Some(AppAction::Quit)
    ));
}

#[test]
fn esc_closes_help() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('?')));
    app.handle_key(key(KeyCode::Esc));
    assert!(matches!(
        app.handle_key(key(KeyCode::Char('q'))),
        Some(AppAction::Quit)
    ));
}

// ---------------------------------------------------------------------------
// ConfirmDelete mode
// ---------------------------------------------------------------------------

#[test]
fn d_enters_confirm_and_enter_deletes() {
    let mut app = App::new();
    app.update_panes(panes(1));
    app.handle_key(key(KeyCode::Char('d')));
    assert!(matches!(
        app.handle_key(key(KeyCode::Enter)),
        Some(AppAction::DeletePane(_))
    ));
}

#[test]
fn esc_in_confirm_delete_cancels_and_returns_to_normal() {
    let mut app = App::new();
    app.update_panes(panes(1));
    app.handle_key(key(KeyCode::Char('d')));
    app.handle_key(key(KeyCode::Esc));
    assert!(matches!(
        app.handle_key(key(KeyCode::Char('q'))),
        Some(AppAction::Quit)
    ));
}

#[test]
fn unrelated_key_in_confirm_delete_keeps_mode() {
    let mut app = App::new();
    app.update_panes(panes(1));
    app.handle_key(key(KeyCode::Char('d')));
    app.handle_key(key(KeyCode::Char('x'))); // unrelated — should stay in ConfirmDelete
    // q is not Quit in ConfirmDelete
    assert!(app.handle_key(key(KeyCode::Char('q'))).is_none());
}

// ---------------------------------------------------------------------------
// Picker mode
// ---------------------------------------------------------------------------

#[test]
fn n_enters_picker_and_esc_exits_to_normal() {
    let mut app = App::new();
    app.handle_key(key(KeyCode::Char('n')));
    // q is not Quit inside the picker
    assert!(app.handle_key(key(KeyCode::Char('q'))).is_none());
    // Esc at session level exits picker
    app.handle_key(key(KeyCode::Esc));
    assert!(matches!(
        app.handle_key(key(KeyCode::Char('q'))),
        Some(AppAction::Quit)
    ));
}

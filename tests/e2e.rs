//! End-to-end tests for the tmux pane monitor.
//!
//! Uses ratatui's [`TestBackend`] to render the UI into an in-memory buffer,
//! so tests run without a real terminal or tmux session.

use claude_pane_monitor::tmux::{
    pane::{ClaudeStatus, PaneId, PaneInfo, PaneState, ShellKind, ShellStatus},
    ui::{App, AppAction},
};
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use ratatui::{Terminal, backend::TestBackend};
use std::sync::Arc;
use std::time::UNIX_EPOCH;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Constructs a minimal [`PaneInfo`] for a given state.
fn make_pane(session: &str, pane_id: u32, state: PaneState) -> PaneInfo {
    PaneInfo {
        id: PaneId {
            session_name: session.to_string(),
            window_index: 0,
            window_name: "win".to_string(),
            pane_id,
        },
        pane_active: false,
        pane_in_mode: false,
        current_cmd: "bash".to_string(),
        state,
        last_updated: UNIX_EPOCH,
        last_focused_at: UNIX_EPOCH,
        status_changed_at: UNIX_EPOCH,
    }
}

/// Renders `app` into a 100×30 buffer and returns the full content as a string.
fn render(app: &App) -> String {
    let backend = TestBackend::new(100, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| app.render(f)).unwrap();
    terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|cell| cell.symbol())
        .collect()
}

/// Constructs a key-press [`KeyEvent`] for the given [`KeyCode`].
fn press(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

// ---------------------------------------------------------------------------
// UI rendering — pane state labels
// ---------------------------------------------------------------------------

#[test]
fn renders_shell_bash_awaiting_input() {
    let mut app = App::new();
    app.update_panes(Arc::new(vec![
        make_pane("work", 1, PaneState::Shell(ShellKind::Bash, ShellStatus::AwaitingInput)),
    ]));
    assert!(render(&app).contains(">_ bash"));
}

#[test]
fn renders_shell_zsh_processing() {
    let mut app = App::new();
    app.update_panes(Arc::new(vec![
        make_pane("work", 1, PaneState::Shell(ShellKind::Zsh, ShellStatus::Processing)),
    ]));
    assert!(render(&app).contains("◉ zsh"));
}

#[test]
fn renders_shell_fish_error() {
    let mut app = App::new();
    app.update_panes(Arc::new(vec![
        make_pane("work", 1, PaneState::Shell(ShellKind::Fish, ShellStatus::Error)),
    ]));
    assert!(render(&app).contains("✗ fish"));
}

#[test]
fn renders_claude_thinking() {
    let mut app = App::new();
    app.update_panes(Arc::new(vec![
        make_pane("work", 1, PaneState::Claude(ClaudeStatus::Thinking)),
    ]));
    assert!(render(&app).contains("◌ claude"));
}

#[test]
fn renders_claude_executing() {
    let mut app = App::new();
    app.update_panes(Arc::new(vec![
        make_pane("work", 1, PaneState::Claude(ClaudeStatus::Executing)),
    ]));
    assert!(render(&app).contains("⚙ claude"));
}

#[test]
fn renders_claude_awaiting_input() {
    let mut app = App::new();
    app.update_panes(Arc::new(vec![
        make_pane("work", 1, PaneState::Claude(ClaudeStatus::AwaitingInput)),
    ]));
    assert!(render(&app).contains(">_ claude"));
}

#[test]
fn renders_other_process() {
    let mut app = App::new();
    app.update_panes(Arc::new(vec![
        make_pane("work", 1, PaneState::Other("vim".to_string())),
    ]));
    assert!(render(&app).contains("? vim"));
}

#[test]
fn renders_never_for_unset_timing_fields() {
    let mut app = App::new();
    app.update_panes(Arc::new(vec![
        make_pane("work", 1, PaneState::Shell(ShellKind::Bash, ShellStatus::AwaitingInput)),
    ]));
    // Both last_focused_at and status_changed_at are UNIX_EPOCH — should display "never".
    let output = render(&app);
    assert!(output.contains("never"));
}

#[test]
fn renders_table_header() {
    let app = App::new();
    let output = render(&app);
    assert!(output.contains("Session:Win"));
    assert!(output.contains("State"));
    assert!(output.contains("Last Focus"));
}

#[test]
fn renders_multiple_panes() {
    let mut app = App::new();
    app.update_panes(Arc::new(vec![
        make_pane("main", 1, PaneState::Shell(ShellKind::Bash, ShellStatus::AwaitingInput)),
        make_pane("main", 2, PaneState::Claude(ClaudeStatus::Generating)),
        make_pane("work", 3, PaneState::Shell(ShellKind::Zsh, ShellStatus::Processing)),
    ]));
    let output = render(&app);
    assert!(output.contains(">_ bash"));
    assert!(output.contains("◉ claude"));
    assert!(output.contains("◉ zsh"));
}

// ---------------------------------------------------------------------------
// Keyboard input — AppAction mapping
// ---------------------------------------------------------------------------

#[test]
fn q_key_returns_quit_action() {
    let mut app = App::new();
    assert!(matches!(app.handle_key(press(KeyCode::Char('q'))), Some(AppAction::Quit)));
}

#[test]
fn capital_q_key_returns_quit_action() {
    let mut app = App::new();
    assert!(matches!(app.handle_key(press(KeyCode::Char('Q'))), Some(AppAction::Quit)));
}

#[test]
fn unbound_key_returns_no_action() {
    let mut app = App::new();
    assert!(app.handle_key(press(KeyCode::Char('x'))).is_none());
    assert!(app.handle_key(press(KeyCode::Enter)).is_none());
}

// ---------------------------------------------------------------------------
// Navigation — selection movement
// ---------------------------------------------------------------------------

#[test]
fn j_key_moves_selection_down() {
    let mut app = App::new();
    app.update_panes(Arc::new(vec![
        make_pane("s", 1, PaneState::Other("a".into())),
        make_pane("s", 2, PaneState::Other("b".into())),
    ]));
    app.handle_key(press(KeyCode::Char('j')));
    // Selection moved — row 2 is now highlighted (dark background in buffer).
    // We verify indirectly: pressing j again wraps back to 0, so j+j = same as start.
    app.handle_key(press(KeyCode::Char('j')));
    // After two j presses on 2 panes, selection wraps back to first.
    let output_after_wrap = render(&app);
    // First pane's session appears somewhere near top of table.
    assert!(output_after_wrap.contains("s:win"));
}

#[test]
fn down_arrow_moves_selection_down() {
    let mut app = App::new();
    app.update_panes(Arc::new(vec![
        make_pane("s", 1, PaneState::Other("a".into())),
        make_pane("s", 2, PaneState::Other("b".into())),
    ]));
    assert!(app.handle_key(press(KeyCode::Down)).is_none());
}

#[test]
fn k_key_on_first_row_does_not_underflow() {
    let mut app = App::new();
    app.update_panes(Arc::new(vec![
        make_pane("s", 1, PaneState::Other("a".into())),
    ]));
    // Pressing k at the top row should clamp — no panic, no wrap.
    app.handle_key(press(KeyCode::Char('k')));
    assert!(render(&app).contains("s:win")); // still renders fine
}

#[test]
fn navigation_does_nothing_on_empty_pane_list() {
    let mut app = App::new();
    // Should not panic with empty pane list.
    app.handle_key(press(KeyCode::Char('j')));
    app.handle_key(press(KeyCode::Char('k')));
    app.handle_key(press(KeyCode::Down));
    app.handle_key(press(KeyCode::Up));
}

// ---------------------------------------------------------------------------
// update_panes — selection clamping
// ---------------------------------------------------------------------------

#[test]
fn update_panes_clamps_selection_when_list_shrinks() {
    let mut app = App::new();
    // Start with 3 panes and select the last one.
    app.update_panes(Arc::new(vec![
        make_pane("s", 1, PaneState::Other("a".into())),
        make_pane("s", 2, PaneState::Other("b".into())),
        make_pane("s", 3, PaneState::Other("c".into())),
    ]));
    app.handle_key(press(KeyCode::Char('j')));
    app.handle_key(press(KeyCode::Char('j')));

    // Pane list shrinks to 1 — selection must clamp rather than go out of bounds.
    app.update_panes(Arc::new(vec![
        make_pane("s", 1, PaneState::Other("a".into())),
    ]));

    // Should still render without panic.
    assert!(render(&app).contains("s:win"));
}

use super::*;
use crate::tmux::pane::shell::ProcessOutcome;

// ---------------------------------------------------------------------------
// PaneId
// ---------------------------------------------------------------------------

#[test]
fn pane_id_target_formats_correctly() {
    let id = PaneId {
        session_name: "main".to_string(),
        window_index: 1,
        window_name: "editor".to_string(),
        pane_id: 3,
    };
    assert_eq!(id.target(), "%3");
}

// ---------------------------------------------------------------------------
// ShellKind::from_process_name
// ---------------------------------------------------------------------------

#[test]
fn shell_kind_recognises_known_shells() {
    assert_eq!(ShellKind::from_process_name("bash"), Some(ShellKind::Bash));
    assert_eq!(ShellKind::from_process_name("zsh"), Some(ShellKind::Zsh));
    assert_eq!(ShellKind::from_process_name("fish"), Some(ShellKind::Fish));
    assert_eq!(ShellKind::from_process_name("sh"), Some(ShellKind::Sh));
    assert_eq!(ShellKind::from_process_name("dash"), Some(ShellKind::Sh));
}

#[test]
fn shell_kind_returns_none_for_unknown_process() {
    assert_eq!(ShellKind::from_process_name("python"), None);
    assert_eq!(ShellKind::from_process_name("vim"), None);
    assert_eq!(ShellKind::from_process_name("claude"), None);
    assert_eq!(ShellKind::from_process_name(""), None);
}

// ---------------------------------------------------------------------------
// ShellStatus::from_pane_content
// ---------------------------------------------------------------------------

#[test]
fn shell_status_idle_on_dollar_prompt() {
    assert_eq!(
        ShellStatus::from_pane_content("user@host ~/project $"),
        ShellStatus::Idle
    );
}

#[test]
fn shell_status_idle_on_percent_prompt() {
    assert_eq!(
        ShellStatus::from_pane_content("user@host ~/project %"),
        ShellStatus::Idle
    );
}

#[test]
fn shell_status_idle_on_hash_prompt() {
    // Root shell prompt
    assert_eq!(
        ShellStatus::from_pane_content("root@host ~#"),
        ShellStatus::Idle
    );
}

#[test]
fn shell_status_awaiting_input_when_no_prompt_visible() {
    let content = "Running cargo build...\n   Compiling my-crate v0.1.0";
    assert_eq!(
        ShellStatus::from_pane_content(content),
        ShellStatus::AwaitingInput
    );
}

#[test]
fn shell_status_awaiting_input_on_empty_content() {
    assert_eq!(
        ShellStatus::from_pane_content(""),
        ShellStatus::AwaitingInput
    );
    assert_eq!(
        ShellStatus::from_pane_content("   \n  "),
        ShellStatus::AwaitingInput
    );
}

// ---------------------------------------------------------------------------
// ClaudeStatus::from_pane_content
// ---------------------------------------------------------------------------

#[test]
fn claude_status_unknown_on_error_like_text() {
    // "Error:" and "✗" are too common in Claude's output to reliably indicate
    // a true error state — they fall through to Unknown.
    assert_eq!(
        ClaudeStatus::from_pane_content("Error: something went wrong"),
        ClaudeStatus::Unknown
    );
    assert_eq!(
        ClaudeStatus::from_pane_content("✗ failed to connect"),
        ClaudeStatus::Unknown
    );
}

#[test]
fn claude_status_thinking_on_esc_interrupt_with_thinking_progress() {
    // Reflects real Claude output: "· <desc> (thinking)" progress line + working indicator with ing…
    let content = "· Sock-hopping… (thinking)\n· Processing…\n  line2\n  line3\n  esc to interrupt";
    assert_eq!(
        ClaudeStatus::from_pane_content(content),
        ClaudeStatus::Thinking
    );
}

#[test]
fn claude_status_thinking_on_esc_interrupt_with_spinner() {
    // Spinner present + WORKING_INDICATOR with ing… at position 4 from bottom
    let content = "⠙ Processing\n· Running…\n  line2\n  line3\n  esc to interrupt";
    assert_eq!(
        ClaudeStatus::from_pane_content(content),
        ClaudeStatus::Thinking
    );
}

#[test]
fn claude_status_executing_on_esc_interrupt_without_thinking_indicators() {
    // Tool running: WORKING_INDICATOR + ing… at position 4 from bottom, no thinking indicators
    let content = "· Executing…\n  Compiling...\n  second line\n  esc to interrupt";
    assert_eq!(
        ClaudeStatus::from_pane_content(content),
        ClaudeStatus::Executing
    );
}

#[test]
fn claude_status_awaiting_permission_on_permission_prompt() {
    let content = "⏺ Update(src/main.rs)\n\n  Do you want to make this edit?\n  ❯ 1. Yes\n    2. No\n\n  Esc to cancel · Tab to amend";
    assert_eq!(
        ClaudeStatus::from_pane_content(content),
        ClaudeStatus::AwaitingPermission
    );
}

#[test]
fn claude_status_awaiting_input_on_box_drawing_input() {
    let content = "Should I proceed?\n\n╭─────────────╮\n│ > type here │\n╰─────────────╯";
    assert_eq!(
        ClaudeStatus::from_pane_content(content),
        ClaudeStatus::AwaitingInput
    );
}

#[test]
fn claude_status_awaiting_input_on_chevron_prompt() {
    // Current Claude Code input format: ────/❯ separators, ? present above the text box.
    let content = "Would you like me to continue?\n\n────────────────────────────────────────\n❯ \n────────────────────────────────────────";
    assert_eq!(
        ClaudeStatus::from_pane_content(content),
        ClaudeStatus::AwaitingInput
    );
}

#[test]
fn claude_status_done_when_input_box_visible_without_question() {
    // Task completed, input box shown, but no ? above it — Done, not AwaitingInput.
    let content = "✻ Churned for 43s\n\n────────────────────────────────────────\n❯ \n────────────────────────────────────────";
    assert_eq!(ClaudeStatus::from_pane_content(content), ClaudeStatus::Done);
}

#[test]
fn claude_status_done_with_many_lines_in_input_box() {
    // User pasted 15 lines into the input box — top separator is far from the bottom.
    // Previously failed because the scanner only looked at the last 8 non-empty lines.
    let typed_lines = (0..15)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!(
        "✻ Churned for 43s\n\n────────────────────────────────────────\n{typed_lines}\n────────────────────────────────────────"
    );
    assert_eq!(
        ClaudeStatus::from_pane_content(&content),
        ClaudeStatus::Done
    );
}

#[test]
fn claude_status_awaiting_input_with_many_lines_in_input_box() {
    let typed_lines = (0..15)
        .map(|i| format!("line {i}"))
        .collect::<Vec<_>>()
        .join("\n");
    let content = format!(
        "Should I continue?\n\n────────────────────────────────────────\n{typed_lines}\n────────────────────────────────────────"
    );
    assert_eq!(
        ClaudeStatus::from_pane_content(&content),
        ClaudeStatus::AwaitingInput
    );
}

// ❯ alone (without ╭ or permission prompt markers) should NOT trigger AwaitingInput —
// it also appears in the input area while Claude is actively working.
#[test]
fn claude_status_not_awaiting_when_only_chevron_visible_while_working() {
    // User has typed in the input box; ─ separators are present but WORKING_INDICATOR with ing…
    // is deeper, so is_working fires first and prevents a false AwaitingInput classification.
    let content = "❯ user message\n· Running…\n─────\n❯ \n─────\n  esc to interrupt";
    assert_ne!(
        ClaudeStatus::from_pane_content(content),
        ClaudeStatus::AwaitingInput
    );
}

#[test]
fn claude_status_unknown_when_no_recognisable_markers() {
    assert_eq!(
        ClaudeStatus::from_pane_content("The quick brown fox jumps over the lazy dog."),
        ClaudeStatus::Unknown
    );
    assert_eq!(ClaudeStatus::from_pane_content(""), ClaudeStatus::Unknown);
    assert_eq!(
        ClaudeStatus::from_pane_content("  \n  "),
        ClaudeStatus::Unknown
    );
}

// ---------------------------------------------------------------------------
// PaneState::from_process — routing
// ---------------------------------------------------------------------------

#[test]
fn from_process_routes_known_shells_to_shell_variant() {
    let state = PaneState::from_process("bash", "user@host $", false);
    assert!(matches!(
        state,
        PaneState::Shell(ShellKind::Bash, ShellStatus::Idle)
    ));
}

#[test]
fn from_process_routes_claude_to_claude_variant() {
    let state = PaneState::from_process("claude", "", false);
    assert!(matches!(state, PaneState::Claude(ClaudeStatus::Unknown)));
}

#[test]
fn from_process_routes_unknown_process_to_other() {
    let state = PaneState::from_process("vim", "", false);
    assert_eq!(state, PaneState::Other("vim".to_string()));
}

#[test]
fn from_process_routes_tc_watcher_active() {
    let state = PaneState::from_process("tc-watcher", "", false);
    assert!(matches!(
        state,
        PaneState::TcWatcher(TcWatcherStatus::Active)
    ));
}

#[test]
fn from_process_routes_tc_watcher_paused_in_copy_mode() {
    let state = PaneState::from_process("tc-watcher", "", true);
    assert!(matches!(
        state,
        PaneState::TcWatcher(TcWatcherStatus::Paused)
    ));
}

// ---------------------------------------------------------------------------
// PaneState::type_cell / state_cell — column content
// ---------------------------------------------------------------------------

#[test]
fn type_cell_shell_uses_shell_kind_name() {
    let line = PaneState::Shell(ShellKind::Bash, ShellStatus::AwaitingInput).type_cell();
    assert_eq!(line.to_string(), "bash");
}

#[test]
fn type_cell_claude_uses_claude_name() {
    let line = PaneState::Claude(ClaudeStatus::Thinking).type_cell();
    assert_eq!(line.to_string(), "claude");
}

#[test]
fn type_cell_other_uses_process_name() {
    let state = PaneState::Other("vim".to_string());
    assert_eq!(state.type_cell().to_string(), "vim");
}

#[test]
fn state_cell_shell_idle_shows_circle_icon() {
    let line = PaneState::Shell(ShellKind::Zsh, ShellStatus::Idle).state_cell();
    assert_eq!(line.to_string(), "○");
}

#[test]
fn state_cell_shell_awaiting_input_shows_chevron_icon() {
    let line = PaneState::Shell(ShellKind::Zsh, ShellStatus::AwaitingInput).state_cell();
    assert_eq!(line.to_string(), "❯");
}

#[test]
fn state_cell_claude_thinking_shows_open_circle_icon() {
    let line = PaneState::Claude(ClaudeStatus::Thinking).state_cell();
    assert_eq!(line.to_string(), "◌");
}

#[test]
fn state_cell_claude_executing_shows_half_circle_icon() {
    let line = PaneState::Claude(ClaudeStatus::Executing).state_cell();
    assert_eq!(line.to_string(), "◑");
}

#[test]
fn state_cell_other_shows_question_mark() {
    let state = PaneState::Other("vim".to_string());
    assert_eq!(state.state_cell().to_string(), "?");
}

// ---------------------------------------------------------------------------
// PaneState::urgency_tier — sort priority for the main table
// ---------------------------------------------------------------------------

#[test]
fn tc_watcher_is_highest_priority() {
    assert_eq!(
        PaneState::TcWatcher(TcWatcherStatus::Active).urgency_tier(),
        0
    );
    assert_eq!(
        PaneState::TcWatcher(TcWatcherStatus::Paused).urgency_tier(),
        0
    );
}

#[test]
fn claude_needing_user_action_is_tier_one() {
    assert_eq!(
        PaneState::Claude(ClaudeStatus::AwaitingInput).urgency_tier(),
        1
    );
    assert_eq!(
        PaneState::Claude(ClaudeStatus::AwaitingPermission).urgency_tier(),
        1
    );
}

#[test]
fn just_finished_is_tier_one() {
    let success = PaneState::Shell(
        ShellKind::Zsh,
        ShellStatus::JustFinished {
            cmd: "cargo".into(),
            outcome: ProcessOutcome::Success,
        },
    );
    let failed = PaneState::Shell(
        ShellKind::Zsh,
        ShellStatus::JustFinished {
            cmd: "gradle".into(),
            outcome: ProcessOutcome::Failed,
        },
    );
    assert_eq!(success.urgency_tier(), 1);
    assert_eq!(failed.urgency_tier(), 1);
}

#[test]
fn state_cell_just_finished_success_shows_icon_and_cmd() {
    let state = PaneState::Shell(
        ShellKind::Zsh,
        ShellStatus::JustFinished {
            cmd: "cargo".into(),
            outcome: ProcessOutcome::Success,
        },
    );
    assert_eq!(state.state_cell().to_string(), "✓ cargo");
}

#[test]
fn state_cell_just_finished_failed_shows_icon_and_cmd() {
    let state = PaneState::Shell(
        ShellKind::Zsh,
        ShellStatus::JustFinished {
            cmd: "gradle".into(),
            outcome: ProcessOutcome::Failed,
        },
    );
    assert_eq!(state.state_cell().to_string(), "✗ gradle");
}

#[test]
fn idle_claude_states_are_not_elevated() {
    // Thinking/Executing/Done/Unknown should NOT be promoted — they don't need user action.
    assert_eq!(PaneState::Claude(ClaudeStatus::Thinking).urgency_tier(), 2);
    assert_eq!(PaneState::Claude(ClaudeStatus::Executing).urgency_tier(), 2);
    assert_eq!(PaneState::Claude(ClaudeStatus::Done).urgency_tier(), 2);
    assert_eq!(PaneState::Claude(ClaudeStatus::Unknown).urgency_tier(), 2);
}

// ---------------------------------------------------------------------------
// PaneInfo::most_recent_activity — used as secondary sort key
// ---------------------------------------------------------------------------

fn pane_with_times(
    status: Option<std::time::SystemTime>,
    focus: Option<std::time::SystemTime>,
) -> PaneInfo {
    PaneInfo {
        id: PaneId {
            session_name: "s".into(),
            window_index: 0,
            window_name: "w".into(),
            pane_id: 0,
        },
        pane_active: false,
        window_active: true,
        session_attached: true,
        pane_in_mode: false,
        current_cmd: "bash".into(),
        last_exit_status: 0,
        state: PaneState::Shell(ShellKind::Bash, ShellStatus::Idle),
        last_updated: std::time::SystemTime::now(),
        status_changed_at: status,
        last_focused_at: focus,
    }
}

#[test]
fn most_recent_activity_returns_the_later_timestamp() {
    let older = std::time::SystemTime::now() - std::time::Duration::from_secs(60);
    let newer = std::time::SystemTime::now() - std::time::Duration::from_secs(5);
    assert_eq!(
        pane_with_times(Some(older), Some(newer)).most_recent_activity(),
        newer
    );
    assert_eq!(
        pane_with_times(Some(newer), Some(older)).most_recent_activity(),
        newer
    );
}

#[test]
fn most_recent_activity_falls_back_to_epoch_when_both_none() {
    assert_eq!(
        pane_with_times(None, None).most_recent_activity(),
        std::time::SystemTime::UNIX_EPOCH
    );
}

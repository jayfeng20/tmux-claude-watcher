use super::*;

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
    assert_eq!(id.target(), "main:1.3");
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
fn shell_status_awaiting_input_on_dollar_prompt() {
    assert_eq!(
        ShellStatus::from_pane_content("user@host ~/project $"),
        ShellStatus::AwaitingInput
    );
}

#[test]
fn shell_status_awaiting_input_on_percent_prompt() {
    assert_eq!(
        ShellStatus::from_pane_content("user@host ~/project %"),
        ShellStatus::AwaitingInput
    );
}

#[test]
fn shell_status_awaiting_input_on_hash_prompt() {
    // Root shell prompt
    assert_eq!(
        ShellStatus::from_pane_content("root@host ~#"),
        ShellStatus::AwaitingInput
    );
}

#[test]
fn shell_status_processing_when_no_prompt_visible() {
    let content = "Running cargo build...\n   Compiling my-crate v0.1.0";
    assert_eq!(
        ShellStatus::from_pane_content(content),
        ShellStatus::Processing
    );
}

#[test]
fn shell_status_error_on_command_not_found() {
    assert_eq!(
        ShellStatus::from_pane_content("bash: foobar: command not found"),
        ShellStatus::Error
    );
}

#[test]
fn shell_status_error_on_zsh_prefix() {
    assert_eq!(
        ShellStatus::from_pane_content("zsh: no such file or directory: foo"),
        ShellStatus::Error
    );
}

#[test]
fn shell_status_idle_on_empty_content() {
    assert_eq!(ShellStatus::from_pane_content(""), ShellStatus::Idle);
    assert_eq!(ShellStatus::from_pane_content("   \n  "), ShellStatus::Idle);
}

// ---------------------------------------------------------------------------
// ClaudeStatus::from_pane_content
// ---------------------------------------------------------------------------

#[test]
fn claude_status_error_on_error_text() {
    assert_eq!(
        ClaudeStatus::from_pane_content("Error: something went wrong"),
        ClaudeStatus::Error
    );
    assert_eq!(
        ClaudeStatus::from_pane_content("✗ failed to connect"),
        ClaudeStatus::Error
    );
}

#[test]
fn claude_status_thinking_on_spinner_char() {
    assert_eq!(
        ClaudeStatus::from_pane_content("⠙ Processing your request"),
        ClaudeStatus::Thinking
    );
}

#[test]
fn claude_status_thinking_on_thinking_text() {
    assert_eq!(
        ClaudeStatus::from_pane_content("Claude is Thinking about your question..."),
        ClaudeStatus::Thinking
    );
}

#[test]
fn claude_status_executing_on_tool_marker() {
    assert_eq!(
        ClaudeStatus::from_pane_content("⏺ Running bash command: ls -la"),
        ClaudeStatus::Executing
    );
}

#[test]
fn claude_status_awaiting_input_on_box_drawing_chars() {
    let content = "Here is the answer.\n\n╭─────────────╮\n│ > type here │\n╰─────────────╯";
    assert_eq!(
        ClaudeStatus::from_pane_content(content),
        ClaudeStatus::AwaitingInput
    );
}

#[test]
fn claude_status_awaiting_input_on_option_prompt() {
    let content = "Do you want to make this edit to foo.rs?\n❯ 1. Yes\n  2. No\n  3. Yes, always";
    assert_eq!(
        ClaudeStatus::from_pane_content(content),
        ClaudeStatus::AwaitingInput
    );
}

#[test]
fn claude_status_generating_when_content_present_with_no_markers() {
    let content = "The quick brown fox jumps over the lazy dog.";
    assert_eq!(
        ClaudeStatus::from_pane_content(content),
        ClaudeStatus::Generating
    );
}

#[test]
fn claude_status_idle_on_empty_content() {
    assert_eq!(ClaudeStatus::from_pane_content(""), ClaudeStatus::Idle);
    assert_eq!(
        ClaudeStatus::from_pane_content("  \n  "),
        ClaudeStatus::Idle
    );
}

// ---------------------------------------------------------------------------
// PaneState::from_process — routing
// ---------------------------------------------------------------------------

#[test]
fn from_process_routes_known_shells_to_shell_variant() {
    let state = PaneState::from_process("bash", "user@host $");
    assert!(matches!(
        state,
        PaneState::Shell(ShellKind::Bash, ShellStatus::AwaitingInput)
    ));
}

#[test]
fn from_process_routes_claude_to_claude_variant() {
    let state = PaneState::from_process("claude", "");
    assert!(matches!(state, PaneState::Claude(ClaudeStatus::Idle)));
}

#[test]
fn from_process_routes_unknown_process_to_other() {
    let state = PaneState::from_process("vim", "");
    assert_eq!(state, PaneState::Other("vim".to_string()));
}

// ---------------------------------------------------------------------------
// PaneState::display — label and colour composition
// ---------------------------------------------------------------------------

#[test]
fn display_shell_bash_awaiting_input() {
    let (label, _) = PaneState::Shell(ShellKind::Bash, ShellStatus::AwaitingInput).display();
    assert_eq!(label, ">_ bash");
}

#[test]
fn display_shell_zsh_processing() {
    let (label, _) = PaneState::Shell(ShellKind::Zsh, ShellStatus::Processing).display();
    assert_eq!(label, "◉ zsh");
}

#[test]
fn display_claude_thinking() {
    let (label, _) = PaneState::Claude(ClaudeStatus::Thinking).display();
    assert_eq!(label, "◌ claude");
}

#[test]
fn display_other_uses_process_name() {
    let (label, _) = PaneState::Other("vim".to_string()).display();
    assert_eq!(label, "? vim");
}

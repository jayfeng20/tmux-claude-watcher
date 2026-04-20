//! Session / window picker overlay.
//!
//! Renders a centred modal list over the main table. The user navigates with
//! j/k or arrow keys and confirms with Enter. Esc goes back one level (or
//! exits the picker entirely at the top level).
//!
//! The picker is intentionally display-only regarding panes: it derives the
//! session and window lists from the live [`PaneInfo`] snapshot so it never
//! holds stale state of its own.

use super::constants::{
    FOOTER_HINT_PICKER, FOOTER_HINT_PICKER_TOP, PICKER_MAX_HEIGHT, PICKER_WIDTH, centered_rect,
};
use super::name_prompt::PendingCreate;
use crate::theme;
use crate::tmux::pane::{PaneInfo, SessionName, WindowName};
use crossterm::event::KeyCode;
use ratatui::{
    Frame,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};
use std::collections::BTreeSet;

// ---------------------------------------------------------------------------
// Public types
// ---------------------------------------------------------------------------

/// The navigation level currently shown in the picker.
pub(super) enum PickerLevel {
    Session,
    Window { session: SessionName },
}

/// What the picker should do after processing a key.
pub(super) enum PickerOutput {
    /// Selection moved or key had no effect — stay in picker.
    Stay,
    /// User selected an existing session — show its windows.
    Descend(PickerState),
    /// User selected "+ New …" — prompt for a name.
    PromptName(PendingCreate),
    /// User selected an existing window — split it immediately, no name needed.
    CreatePane {
        session: SessionName,
        window: WindowName,
    },
    /// Esc at the window level — rebuild the session picker.
    LevelUp,
    /// Esc at the session level — exit the picker entirely.
    Exit,
}

pub(super) struct PickerState {
    pub(super) level: PickerLevel,
    items: Vec<PickerItem>,
    pub(super) selected: usize,
}

// ---------------------------------------------------------------------------
// Private types
// ---------------------------------------------------------------------------

enum PickerItem {
    ExistingSession(SessionName),
    ExistingWindow(WindowName),
    CreateNewSession,
    CreateNewWindow,
}

impl PickerItem {
    fn label(&self) -> &str {
        match self {
            PickerItem::ExistingSession(s) => s.as_ref(),
            PickerItem::ExistingWindow(w) => w.as_ref(),
            PickerItem::CreateNewSession => "+ New session",
            PickerItem::CreateNewWindow => "+ New window",
        }
    }

    fn is_create_new(&self) -> bool {
        matches!(
            self,
            PickerItem::CreateNewSession | PickerItem::CreateNewWindow
        )
    }
}

// ---------------------------------------------------------------------------
// PickerState impl
// ---------------------------------------------------------------------------

impl PickerState {
    /// Build a session-level picker from the live pane snapshot.
    pub(super) fn for_sessions(panes: &[PaneInfo]) -> Self {
        let sessions: BTreeSet<&str> = panes.iter().map(|p| p.id.session_name.as_str()).collect();
        let mut items: Vec<PickerItem> = sessions
            .into_iter()
            .map(|s| PickerItem::ExistingSession(s.into()))
            .collect();
        items.push(PickerItem::CreateNewSession);
        PickerState {
            level: PickerLevel::Session,
            items,
            selected: 0,
        }
    }

    /// Build a window-level picker for the given session.
    pub(super) fn for_windows(session: SessionName, panes: &[PaneInfo]) -> Self {
        let windows: BTreeSet<(u32, &str)> = panes
            .iter()
            .filter(|p| p.id.session_name == session.as_ref())
            .map(|p| (p.id.window_index, p.id.window_name.as_str()))
            .collect();
        let mut items: Vec<PickerItem> = windows
            .into_iter()
            .map(|(_, name)| PickerItem::ExistingWindow(name.into()))
            .collect();
        items.push(PickerItem::CreateNewWindow);
        PickerState {
            level: PickerLevel::Window { session },
            items,
            selected: 0,
        }
    }

    pub(super) fn handle_key(&mut self, code: KeyCode, panes: &[PaneInfo]) -> PickerOutput {
        match code {
            KeyCode::Char('j') | KeyCode::Down => {
                self.selected = (self.selected + 1).min(self.items.len().saturating_sub(1));
                PickerOutput::Stay
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.selected = self.selected.saturating_sub(1);
                PickerOutput::Stay
            }
            KeyCode::Enter => self.commit(panes),
            KeyCode::Esc => match self.level {
                PickerLevel::Session => PickerOutput::Exit,
                PickerLevel::Window { .. } => PickerOutput::LevelUp,
            },
            _ => PickerOutput::Stay,
        }
    }

    fn commit(&self, panes: &[PaneInfo]) -> PickerOutput {
        let Some(item) = self.items.get(self.selected) else {
            return PickerOutput::Stay;
        };
        match item {
            PickerItem::ExistingSession(session) => {
                PickerOutput::Descend(Self::for_windows(session.clone(), panes))
            }
            PickerItem::CreateNewSession => PickerOutput::PromptName(PendingCreate::Session),
            PickerItem::ExistingWindow(window) => {
                let PickerLevel::Window { session } = &self.level else {
                    return PickerOutput::Stay;
                };
                PickerOutput::CreatePane {
                    session: session.clone(),
                    window: window.clone(),
                }
            }
            PickerItem::CreateNewWindow => {
                let PickerLevel::Window { session } = &self.level else {
                    return PickerOutput::Stay;
                };
                PickerOutput::PromptName(PendingCreate::Window {
                    session: session.clone(),
                })
            }
        }
    }

    pub(super) fn title(&self) -> String {
        match &self.level {
            PickerLevel::Session => "Sessions".to_owned(),
            PickerLevel::Window { session } => format!("Windows — {session}"),
        }
    }

    pub(super) fn footer_hint(&self) -> &'static str {
        match self.level {
            PickerLevel::Session => FOOTER_HINT_PICKER_TOP,
            PickerLevel::Window { .. } => FOOTER_HINT_PICKER,
        }
    }
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

pub(super) fn render(frame: &mut Frame, state: &PickerState) {
    let height = (state.items.len() as u16 + 2).min(PICKER_MAX_HEIGHT);
    let area = centered_rect(PICKER_WIDTH, height, frame.area());
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = state
        .items
        .iter()
        .map(|item| {
            let style = if item.is_create_new() {
                Style::default().fg(theme::TEAL)
            } else {
                Style::default()
            };
            ListItem::new(Line::from(Span::styled(
                format!(" {}", item.label()),
                style,
            )))
        })
        .collect();

    let list = List::new(items)
        .highlight_style(Style::default().bg(theme::SELECTED_BG))
        .block(
            Block::default()
                .title(state.title())
                .borders(Borders::ALL)
                .style(Style::default().bg(Color::Black)),
        );

    let mut list_state = ListState::default().with_selected(Some(state.selected));
    frame.render_stateful_widget(list, area, &mut list_state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmux::pane::{PaneId, PaneInfo, PaneState, ShellKind, ShellStatus};
    use crossterm::event::KeyCode;
    use std::time::SystemTime;

    fn pane(session: &str, window_index: u32, window_name: &str) -> PaneInfo {
        PaneInfo {
            id: PaneId {
                session_name: session.to_string(),
                window_index,
                window_name: window_name.to_string(),
                pane_id: window_index,
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
        }
    }

    fn two_session_panes() -> Vec<PaneInfo> {
        vec![
            pane("alpha", 0, "win"),
            pane("beta", 0, "main"),
            pane("beta", 1, "logs"),
        ]
    }

    // -----------------------------------------------------------------------
    // for_sessions / for_windows construction
    // -----------------------------------------------------------------------

    #[test]
    fn for_sessions_produces_one_item_per_unique_session_plus_create_new() {
        let state = PickerState::for_sessions(&two_session_panes());
        // alpha + beta + CreateNewSession = 3
        assert_eq!(state.items.len(), 3);
        assert!(matches!(
            state.items.last(),
            Some(PickerItem::CreateNewSession)
        ));
    }

    #[test]
    fn for_sessions_items_are_sorted_alphabetically() {
        let state = PickerState::for_sessions(&two_session_panes());
        let PickerItem::ExistingSession(first) = &state.items[0] else {
            panic!()
        };
        assert_eq!(first.as_ref(), "alpha");
    }

    #[test]
    fn for_windows_only_includes_windows_from_the_given_session() {
        let state = PickerState::for_windows("beta".into(), &two_session_panes());
        // beta has 2 windows + CreateNewWindow = 3
        assert_eq!(state.items.len(), 3);
        assert!(
            state
                .items
                .iter()
                .all(|item| !matches!(item, PickerItem::ExistingSession(_)))
        );
    }

    // -----------------------------------------------------------------------
    // Navigation — clamping at boundaries
    // -----------------------------------------------------------------------

    #[test]
    fn navigate_down_clamps_at_last_item() {
        let mut state = PickerState::for_sessions(&two_session_panes());
        let last = state.items.len() - 1;
        for _ in 0..10 {
            state.handle_key(KeyCode::Char('j'), &[]);
        }
        assert_eq!(state.selected, last);
    }

    #[test]
    fn navigate_up_clamps_at_zero() {
        let mut state = PickerState::for_sessions(&two_session_panes());
        for _ in 0..10 {
            state.handle_key(KeyCode::Char('k'), &[]);
        }
        assert_eq!(state.selected, 0);
    }

    // -----------------------------------------------------------------------
    // commit — routes to correct output per item type
    // -----------------------------------------------------------------------

    #[test]
    fn commit_existing_session_descends_to_window_picker() {
        let panes = two_session_panes();
        let mut state = PickerState::for_sessions(&panes);
        state.selected = 0; // "alpha"
        let out = state.handle_key(KeyCode::Enter, &panes);
        assert!(matches!(out, PickerOutput::Descend(_)));
    }

    #[test]
    fn commit_create_new_session_prompts_for_name() {
        let panes = two_session_panes();
        let mut state = PickerState::for_sessions(&panes);
        state.selected = state.items.len() - 1; // CreateNewSession
        let out = state.handle_key(KeyCode::Enter, &panes);
        assert!(matches!(
            out,
            PickerOutput::PromptName(PendingCreate::Session)
        ));
    }

    #[test]
    fn commit_existing_window_emits_create_pane() {
        let panes = two_session_panes();
        let mut state = PickerState::for_windows("beta".into(), &panes);
        state.selected = 0; // first window of "beta"
        let out = state.handle_key(KeyCode::Enter, &panes);
        assert!(matches!(out, PickerOutput::CreatePane { .. }));
    }

    #[test]
    fn commit_create_new_window_prompts_with_session_context() {
        let panes = two_session_panes();
        let mut state = PickerState::for_windows("beta".into(), &panes);
        state.selected = state.items.len() - 1; // CreateNewWindow
        let out = state.handle_key(KeyCode::Enter, &panes);
        assert!(
            matches!(out, PickerOutput::PromptName(PendingCreate::Window { ref session }) if session.as_ref() == "beta")
        );
    }

    // -----------------------------------------------------------------------
    // Esc — level-up vs exit
    // -----------------------------------------------------------------------

    #[test]
    fn esc_at_session_level_exits_picker() {
        let mut state = PickerState::for_sessions(&two_session_panes());
        assert!(matches!(
            state.handle_key(KeyCode::Esc, &[]),
            PickerOutput::Exit
        ));
    }

    #[test]
    fn esc_at_window_level_goes_up_one_level() {
        let mut state = PickerState::for_windows("beta".into(), &two_session_panes());
        assert!(matches!(
            state.handle_key(KeyCode::Esc, &[]),
            PickerOutput::LevelUp
        ));
    }
}

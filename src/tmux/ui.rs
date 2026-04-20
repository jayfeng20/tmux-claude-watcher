//! Terminal UI for the tmux pane monitor.
//!
//! [`App`] owns all UI state and is the single orchestration point.
//! Sub-modules each own one widget or concept:
//!
//! | Module        | Responsibility                              |
//! |---------------|---------------------------------------------|
//! | `constants`   | Every magic number, key label, hint string  |
//! | `table`       | Main pane-list widget                       |
//! | `footer`      | One-line footer bar (hints / input / error) |
//! | `help`        | Help overlay                                |
//! | `picker`      | Session/window picker overlay               |
//! | `name_prompt` | Single-field name input after "+ New …"     |
//!
//! [`App::handle_key`] maps raw key events to [`AppAction`] values and
//! drives all mode transitions. `main` dispatches the returned actions.

mod constants;
mod footer;
mod help;
mod name_prompt;
mod picker;
mod table;

#[cfg(test)]
mod tests;

use crate::tmux::pane::{PaneId, PaneInfo, SessionName, WindowName};
use constants::{FOOTER_CONFIRM_SUFFIX, FOOTER_HINT};
use crossterm::event::{KeyCode, KeyEvent};
use footer::FooterContent;
use name_prompt::{NamePromptResult, NamePromptState, PendingCreate};
use picker::{PickerOutput, PickerState};
use ratatui::{
    Frame,
    layout::{Constraint, Layout},
};
use std::sync::Arc;
use std::time::Instant;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// High-level actions produced by [`App::handle_key`] and dispatched by `main`.
pub enum AppAction {
    Quit,
    JumpToPane(PaneId),
    NewSession {
        name: SessionName,
    },
    NewWindow {
        session: SessionName,
        name: WindowName,
    },
    SplitPane {
        session: SessionName,
        window: WindowName,
    },
    DeletePane(PaneId),
}

// ---------------------------------------------------------------------------
// Application mode
// ---------------------------------------------------------------------------

/// All mutually exclusive UI states.
///
/// A single enum makes invalid combinations unrepresentable — the type system
/// prevents being in e.g. `Help` and `ConfirmDelete` simultaneously.
enum AppMode {
    Normal,
    Help,
    ConfirmDelete,
    Picker(PickerState),
    NamePrompt(NamePromptState),
}

// ---------------------------------------------------------------------------
// App
// ---------------------------------------------------------------------------

/// Root application state. All rendering and input handling flows through here.
pub struct App {
    pub panes: Arc<Vec<PaneInfo>>,
    selected: usize,
    error: Option<(String, Instant)>,
    mode: AppMode,
    /// Footer hint
    hint: String,
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}

impl App {
    pub fn new() -> Self {
        App {
            panes: Arc::new(vec![]),
            selected: 0,
            error: None,
            mode: AppMode::Normal,
            hint: FOOTER_HINT.to_string(),
        }
    }

    /// Replaces the pane snapshot and clamps the selection if needed.
    pub fn update_panes(&mut self, panes: Arc<Vec<PaneInfo>>) {
        self.panes = panes;
        if self.selected >= self.panes.len() {
            self.selected = self.panes.len().saturating_sub(1);
        }
    }

    /// Surfaces an error message in the footer for a short TTL.
    pub fn set_error(&mut self, msg: String) {
        self.error = Some((msg, Instant::now()));
    }

    /// Appends `"  prefix+<key> return"` to the footer hint line.
    pub fn set_return_key(&mut self, key: char) {
        self.hint.push_str(&format!("  prefix+{key} return"));
    }

    // -----------------------------------------------------------------------
    // Input handling
    // -----------------------------------------------------------------------

    /// Translates a raw key event into an [`AppAction`], driving all mode
    /// transitions as a side effect. Returns `None` for unbound keys.
    pub fn handle_key(&mut self, key: KeyEvent) -> Option<AppAction> {
        // Take ownership of the current mode, defaulting back to Normal so
        // every arm only needs to set self.mode when it deviates.
        let mode = std::mem::replace(&mut self.mode, AppMode::Normal);

        match mode {
            AppMode::Normal => self.handle_normal(key.code),

            AppMode::Help => {
                if !matches!(key.code, KeyCode::Char('?') | KeyCode::Esc) {
                    self.mode = AppMode::Help;
                }
                None
            }

            AppMode::ConfirmDelete => match key.code {
                KeyCode::Enter => self
                    .panes
                    .get(self.selected)
                    .map(|p| AppAction::DeletePane(p.id.clone())),
                KeyCode::Esc => None,
                _ => {
                    self.mode = AppMode::ConfirmDelete;
                    None
                }
            },

            AppMode::Picker(mut state) => match state.handle_key(key.code, &self.panes) {
                PickerOutput::Stay => {
                    self.mode = AppMode::Picker(state);
                    None
                }
                PickerOutput::Descend(next) => {
                    self.mode = AppMode::Picker(next);
                    None
                }
                PickerOutput::PromptName(pending) => {
                    self.mode = AppMode::NamePrompt(NamePromptState::new(pending));
                    None
                }
                PickerOutput::CreatePane { session, window } => {
                    Some(AppAction::SplitPane { session, window })
                }
                PickerOutput::LevelUp => {
                    self.mode = AppMode::Picker(PickerState::for_sessions(&self.panes));
                    None
                }
                PickerOutput::Exit => None,
            },

            AppMode::NamePrompt(mut state) => match state.handle_key(key.code) {
                NamePromptResult::Continue => {
                    self.mode = AppMode::NamePrompt(state);
                    None
                }
                NamePromptResult::Done => {
                    let name = state.buf;
                    Some(match state.pending {
                        PendingCreate::Session => AppAction::NewSession { name: name.into() },
                        PendingCreate::Window { session } => AppAction::NewWindow {
                            session,
                            name: name.into(),
                        },
                    })
                }
                NamePromptResult::Cancel => {
                    self.mode = AppMode::Picker(match state.pending {
                        PendingCreate::Session => PickerState::for_sessions(&self.panes),
                        PendingCreate::Window { session } => {
                            PickerState::for_windows(session, &self.panes)
                        }
                    });
                    None
                }
            },
        }
    }

    fn handle_normal(&mut self, code: KeyCode) -> Option<AppAction> {
        match code {
            KeyCode::Char('?') => {
                self.mode = AppMode::Help;
                None
            }
            KeyCode::Char('q') | KeyCode::Char('Q') => Some(AppAction::Quit),
            KeyCode::Down | KeyCode::Char('j') => {
                self.next();
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.prev();
                None
            }
            KeyCode::Enter => self
                .panes
                .get(self.selected)
                .map(|p| AppAction::JumpToPane(p.id.clone())),
            KeyCode::Char('n') => {
                self.mode = AppMode::Picker(PickerState::for_sessions(&self.panes));
                None
            }
            KeyCode::Char('d') => {
                if self.panes.get(self.selected).is_some() {
                    self.mode = AppMode::ConfirmDelete;
                }
                None
            }
            _ => None,
        }
    }

    fn next(&mut self) {
        if !self.panes.is_empty() {
            self.selected = (self.selected + 1) % self.panes.len();
        }
    }

    fn prev(&mut self) {
        if !self.panes.is_empty() {
            self.selected = self.selected.saturating_sub(1);
        }
    }

    // -----------------------------------------------------------------------
    // Rendering
    // -----------------------------------------------------------------------

    /// Renders the full UI: pane table, footer, and any active overlay.
    pub fn render(&self, frame: &mut Frame) {
        let [table_area, footer_area] =
            Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).areas(frame.area());

        table::render(frame, table_area, &self.panes, self.selected);
        footer::render(frame, footer_area, self.footer_content());

        match &self.mode {
            AppMode::Help => help::render(frame),
            AppMode::Picker(state) => picker::render(frame, state),
            _ => {}
        }
    }

    fn footer_content(&self) -> FooterContent<'_> {
        match &self.mode {
            AppMode::Normal | AppMode::Help => FooterContent::Normal {
                hint: &self.hint,
                error: self.error.as_ref(),
            },
            AppMode::ConfirmDelete => {
                let target = self
                    .panes
                    .get(self.selected)
                    .map(|p| p.id.target())
                    .unwrap_or_default();
                FooterContent::Confirm(format!("Kill {target}{FOOTER_CONFIRM_SUFFIX}"))
            }
            AppMode::Picker(state) => FooterContent::Hint(state.footer_hint()),
            AppMode::NamePrompt(state) => FooterContent::Input {
                prompt: state.pending.prompt(),
                buf: &state.buf,
            },
        }
    }
}

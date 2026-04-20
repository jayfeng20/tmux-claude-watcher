//! Footer bar.
//!
//! Renders one of four states depending on the current [`AppMode`]:
//! - Normal: key-hint line, or a timed error banner after a failed action.
//! - Hint: a static string (used by picker modes).
//! - Input: prompt label + live text buffer + cursor block.
//! - Confirm: destructive-action confirmation with target name.

use super::constants::ERROR_TTL;
use crate::theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::Paragraph,
};
use std::time::Instant;

pub(super) enum FooterContent<'a> {
    /// Normal operation: show key hints, or a timed error if one is active.
    Normal {
        hint: &'a str,
        error: Option<&'a (String, Instant)>,
    },
    /// A static hint string — used when a picker overlay is open.
    Hint(&'static str),
    /// Active text input: prompt label + buffer with a trailing cursor block.
    Input { prompt: &'static str, buf: &'a str },
    /// Destructive-action confirmation with the formatted target + key hints.
    Confirm(String),
}

pub(super) fn render(frame: &mut Frame, area: Rect, content: FooterContent<'_>) {
    let widget = match content {
        FooterContent::Normal { hint, error } => {
            let active_error = error
                .filter(|(_, set_at)| set_at.elapsed() < ERROR_TTL)
                .map(|(msg, _)| msg.as_str());
            if let Some(msg) = active_error {
                Paragraph::new(Span::raw(format!("error: {msg}")))
                    .style(Style::default().fg(Color::Red))
            } else {
                Paragraph::new(Span::raw(hint)).style(Style::default().fg(theme::DIM))
            }
        }
        FooterContent::Hint(text) => {
            Paragraph::new(Span::raw(text)).style(Style::default().fg(theme::DIM))
        }
        FooterContent::Input { prompt, buf } => {
            Paragraph::new(Span::raw(format!("{prompt}{buf}▌")))
                .style(Style::default().fg(theme::TEAL))
        }
        FooterContent::Confirm(text) => {
            Paragraph::new(Span::raw(text)).style(Style::default().fg(theme::YELLOW))
        }
    };
    frame.render_widget(widget, area);
}

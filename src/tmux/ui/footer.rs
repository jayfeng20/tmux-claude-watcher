//! Footer bar.
//!
//! Normally shows the key-hint line. Switches to a red error banner for
//! `ERROR_TTL` seconds after a failed action (e.g. jumping to a dead pane),
//! then reverts automatically on the next render.

use super::constants::{ERROR_TTL, FOOTER_HINT};
use crate::theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Style},
    text::Span,
    widgets::Paragraph,
};
use std::time::Instant;

pub(super) fn render(frame: &mut Frame, area: Rect, error: Option<&(String, Instant)>) {
    let active_error = error
        .filter(|(_, set_at)| set_at.elapsed() < ERROR_TTL)
        .map(|(msg, _)| msg.as_str());

    let widget = if let Some(msg) = active_error {
        Paragraph::new(Span::raw(format!("error: {msg}"))).style(Style::default().fg(Color::Red))
    } else {
        Paragraph::new(Span::raw(FOOTER_HINT)).style(Style::default().fg(theme::DIM))
    };

    frame.render_widget(widget, area);
}

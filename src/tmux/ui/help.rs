//! Help overlay.
//!
//! A centred modal panel rendered on top of the table when the user presses
//! `?`. Divided into three sections — state icons, column reference, and key
//! bindings — each built by its own function and assembled in `render`.

use super::constants::{
    HELP_COL_LABEL_PAD, HELP_HEIGHT, HELP_ICON_DESC_PAD, HELP_KEY_LABEL_PAD, HELP_WIDTH,
};
use crate::theme;
use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
};

pub(super) fn render(frame: &mut Frame) {
    let area = centered_rect(HELP_WIDTH, HELP_HEIGHT, frame.area());
    frame.render_widget(Clear, area);

    let mut lines = Vec::new();
    lines.extend(state_icons());
    lines.push(Line::raw(""));
    lines.extend(columns());
    lines.push(Line::raw(""));
    lines.extend(key_bindings());
    lines.push(Line::raw(""));
    lines.push(close_hint());

    let help = Paragraph::new(lines).block(
        Block::default()
            .title("Help")
            .borders(Borders::ALL)
            .style(Style::default().bg(Color::Black)),
    );
    frame.render_widget(help, area);
}

fn state_icons() -> Vec<Line<'static>> {
    let dim = Style::default().fg(theme::DIM);
    let bold = Style::default().add_modifier(Modifier::BOLD);

    vec![
        Line::from(Span::styled("State icons", bold)),
        icon(theme::PEACH, "◌ ", "Thinking", Some(theme::CLAUDE_LABEL)),
        icon(theme::YELLOW, "◑ ", "Executing", Some(theme::CLAUDE_LABEL)),
        icon(theme::RED, "❯ ", "Awaiting input", None),
        icon(
            theme::RED,
            "! ",
            "Awaiting perm.",
            Some(theme::CLAUDE_LABEL),
        ),
        icon(theme::GREEN, "✓ ", "Done", Some(theme::CLAUDE_LABEL)),
        icon(theme::GREEN, "○ ", "Idle", Some(theme::SHELL_LABEL)),
        icon(theme::RED, "✗ ", "Error", Some(theme::SHELL_LABEL)),
        Line::from(vec![
            Span::styled("? ", dim),
            Span::raw(format!("{:<width$}", "Unknown", width = HELP_ICON_DESC_PAD)),
        ]),
    ]
}

/// One state-icon row: colored icon, plain description padded to `HELP_ICON_DESC_PAD`,
/// and an optional colored process-type tag.
fn icon(
    icon_color: ratatui::style::Color,
    symbol: &'static str,
    label: &'static str,
    tag: Option<ratatui::style::Color>,
) -> Line<'static> {
    let padded = format!("{:<width$}", label, width = HELP_ICON_DESC_PAD);
    let mut spans = vec![
        Span::styled(symbol, Style::default().fg(icon_color)),
        Span::raw(padded),
    ];
    if let Some(color) = tag {
        let text = if color == theme::CLAUDE_LABEL {
            "(claude)"
        } else {
            "(shell) "
        };
        spans.push(Span::styled(text, Style::default().fg(color)));
    }
    Line::from(spans)
}

fn columns() -> Vec<Line<'static>> {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    vec![
        Line::from(Span::styled("Columns", bold)),
        col("ID", "session:window.pane"),
        col("Type", "process (zsh, claude, …)"),
        col("State", "activity icon"),
        col("Active", "focused pane in window"),
        col("Last Updated", "time since state changed"),
    ]
}

fn key_bindings() -> Vec<Line<'static>> {
    let bold = Style::default().add_modifier(Modifier::BOLD);
    vec![
        Line::from(Span::styled("Key bindings", bold)),
        binding("↵ Enter", "jump to pane"),
        binding("↑↓ / jk", "navigate"),
        binding("q", "quit"),
    ]
}

fn close_hint() -> Line<'static> {
    Line::from(Span::styled(
        "? or Esc to close",
        Style::default().fg(theme::SUBTLE),
    ))
}

/// Column name (SAPPHIRE, padded to `HELP_COL_LABEL_PAD`) followed by its definition (white).
fn col(name: &'static str, definition: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{:<width$}", name, width = HELP_COL_LABEL_PAD),
            Style::default().fg(theme::SAPPHIRE),
        ),
        Span::raw(definition),
    ])
}

/// Key name (TEAL, padded to `HELP_KEY_LABEL_PAD`) followed by its description (white).
fn binding(key: &'static str, description: &'static str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("{:<width$}", key, width = HELP_KEY_LABEL_PAD),
            Style::default().fg(theme::TEAL),
        ),
        Span::raw(description),
    ])
}

fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

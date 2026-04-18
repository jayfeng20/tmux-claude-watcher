//! Help overlay.
//!
//! A centred modal panel rendered on top of the table when the user presses
//! `?`. Divided into three sections — state icons, column reference, and key
//! bindings — each built by its own function and assembled in `render`.

use super::constants::{
    HELP_COL_LABEL_PAD, HELP_HEIGHT, HELP_ICON_DESC_PAD, HELP_KEY_LABEL_PAD, HELP_TAG_WIDTH,
    HELP_WIDTH,
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
    let bold = Style::default().add_modifier(Modifier::BOLD);

    vec![
        Line::from(Span::styled("State icons", bold)),
        icon(theme::ICON_THINKING, "Thinking", Some(theme::CLAUDE_LABEL)),
        icon(
            theme::ICON_EXECUTING,
            "Executing",
            Some(theme::CLAUDE_LABEL),
        ),
        icon(theme::ICON_AWAITING_INPUT, "Awaiting input", None),
        icon(
            theme::ICON_AWAITING_PERMISSION,
            "Awaiting perm.",
            Some(theme::CLAUDE_LABEL),
        ),
        icon(theme::ICON_DONE, "Done", Some(theme::CLAUDE_LABEL)),
        icon(theme::ICON_IDLE, "Idle", Some(theme::SHELL_LABEL)),
        icon(theme::ICON_ERROR, "Error", Some(theme::SHELL_LABEL)),
        icon(theme::ICON_IDLE, "Active", Some(theme::TC_WATCHER_LABEL)),
        icon(theme::ICON_PAUSED, "Paused", Some(theme::TC_WATCHER_LABEL)),
        icon(theme::ICON_UNKNOWN, "Unknown", None),
    ]
}

/// One state-icon row: colored icon, plain description padded to `HELP_ICON_DESC_PAD`,
/// and an optional colored process-type tag.
fn icon(
    state: (&'static str, ratatui::style::Color),
    label: &'static str,
    tag: Option<ratatui::style::Color>,
) -> Line<'static> {
    let symbol = format!("{} ", state.0);
    let padded = format!("{:<width$}", label, width = HELP_ICON_DESC_PAD);
    let mut spans = vec![
        Span::styled(symbol, Style::default().fg(state.1)),
        Span::raw(padded),
    ];
    if let Some(color) = tag {
        let label = if color == theme::CLAUDE_LABEL {
            "(claude)"
        } else if color == theme::SHELL_LABEL {
            "(shell)"
        } else {
            "(tc-watcher)"
        };
        spans.push(Span::styled(
            format!("{:<width$}", label, width = HELP_TAG_WIDTH),
            Style::default().fg(color),
        ));
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

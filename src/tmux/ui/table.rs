//! Pane table widget.
//!
//! Renders the main table of all active tmux panes with columns for ID,
//! process type, state icon, active status, and time since last state change.

use super::constants::{
    COL_ACTIVE_WIDTH, COL_ID_WIDTH, COL_LAST_UPDATED_WIDTH, COL_STATE_WIDTH, COL_TYPE_WIDTH,
};
use crate::theme;
use crate::tmux::pane::PaneInfo;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Cell, Row, Table},
};
use std::time::SystemTime;

pub(super) fn render(frame: &mut Frame, area: Rect, panes: &[PaneInfo], selected: usize) {
    let header = Row::new(["ID", "Type", "State", "Active", "Last Updated"])
        .style(Style::default().add_modifier(Modifier::BOLD));

    let rows: Vec<Row> = panes
        .iter()
        .enumerate()
        .map(|(i, pane)| {
            let style = if i == selected {
                Style::default().bg(theme::SELECTED_BG)
            } else {
                Style::default()
            };
            // A pane is truly focused only when all three tmux flags align:
            // session_attached, window_active, and pane_active.
            let is_active = pane.session_attached && pane.window_active && pane.pane_active;
            let (active_label, active_color) = if is_active {
                ("yes", theme::GREEN)
            } else {
                ("no", theme::DIM)
            };
            Row::new(vec![
                Cell::from(pane.id.to_string()),
                Cell::from(pane.state.type_cell()),
                Cell::from(pane.state.state_cell()),
                Cell::from(active_label).style(Style::default().fg(active_color)),
                Cell::from(format_ago(pane.status_changed_at)),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(COL_ID_WIDTH),
            Constraint::Length(COL_TYPE_WIDTH),
            Constraint::Length(COL_STATE_WIDTH),
            Constraint::Length(COL_ACTIVE_WIDTH),
            Constraint::Length(COL_LAST_UPDATED_WIDTH),
        ],
    )
    .header(header)
    .block(
        Block::default()
            .title("Tmux Pane Monitor")
            .borders(Borders::ALL),
    );

    frame.render_widget(table, area);
}

fn format_ago(t: Option<SystemTime>) -> String {
    let t = match t {
        None => return "never".to_string(),
        Some(t) => t,
    };
    match t.elapsed() {
        Ok(d) if d.as_secs() < 60 => format!("{}s ago", d.as_secs()),
        Ok(d) => format!("{}m ago", d.as_secs() / 60),
        Err(_) => "—".to_string(),
    }
}

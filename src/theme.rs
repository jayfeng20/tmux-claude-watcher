//! Colour palette and semantic aliases.
//!
//! The palette section defines raw Catppuccin Mocha values. The semantic
//! section maps roles (e.g. process-type labels) to palette entries so that
//! the table and help panel stay in sync through a single definition.

use ratatui::style::Color;

// ---------------------------------------------------------------------------
// Palette — raw colour values
// ---------------------------------------------------------------------------

pub const OVERLAY0: Color = Color::Rgb(108, 112, 134);
pub const SUBTEXT0: Color = Color::Rgb(166, 173, 200);
pub const SURFACE1: Color = Color::Rgb(69, 71, 90);
pub const SAPPHIRE: Color = Color::Rgb(116, 199, 236);
pub const TEAL: Color = Color::Rgb(148, 226, 213);
pub const PEACH: Color = Color::Rgb(250, 179, 135);
pub const MAUVE: Color = Color::Rgb(203, 166, 247);
pub const YELLOW: Color = Color::Rgb(249, 226, 175);
pub const GREEN: Color = Color::Rgb(166, 227, 161);
pub const RED: Color = Color::Rgb(243, 139, 168);

// ---------------------------------------------------------------------------
// Semantic aliases — centralised so all UI files stay in sync.
// ---------------------------------------------------------------------------

/// Process-type label for Claude panes (table Type column + help panel tags).
pub const CLAUDE_LABEL: Color = PEACH;
/// Process-type label for shell panes (table Type column + help panel tags).
pub const SHELL_LABEL: Color = TEAL;
/// Process-type label for tc-watcher panes (table Type column + help panel tags).
pub const TC_WATCHER_LABEL: Color = MAUVE;
/// Dim/inactive/unknown — "no" active, unknown state, unrecognised process.
pub const DIM: Color = OVERLAY0;
/// Selected row background.
pub const SELECTED_BG: Color = SURFACE1;
/// Subtle secondary text — hints, close prompts, unimportant labels.
pub const SUBTLE: Color = SUBTEXT0;

// ---------------------------------------------------------------------------
// State icons — (symbol, colour) pairs shared by pane display() and help panel.
// ---------------------------------------------------------------------------

pub const ICON_IDLE: (&str, Color) = ("○", GREEN);
pub const ICON_AWAITING_INPUT: (&str, Color) = ("❯", RED);
pub const ICON_ERROR: (&str, Color) = ("✗", RED);
pub const ICON_THINKING: (&str, Color) = ("◌", PEACH);
pub const ICON_EXECUTING: (&str, Color) = ("◑", YELLOW);
pub const ICON_DONE: (&str, Color) = ("✓", GREEN);
pub const ICON_AWAITING_PERMISSION: (&str, Color) = ("!", RED);
pub const ICON_UNKNOWN: (&str, Color) = ("?", DIM);
pub const ICON_PAUSED: (&str, Color) = ("⏸", YELLOW);
pub const ICON_STOPPED: (&str, Color) = ("⏹", RED);

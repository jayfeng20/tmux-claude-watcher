//! Layout and timing constants for the TUI.
//!
//! Centralises every magic number used across the UI sub-modules so that
//! resizing a column or tweaking a timeout is a one-line change.

use std::time::Duration;

// ---------------------------------------------------------------------------
// Table column widths
// ---------------------------------------------------------------------------

pub const COL_ID_WIDTH: u16 = 20;
pub const COL_TYPE_WIDTH: u16 = 10;
pub const COL_STATE_WIDTH: u16 = 6;
pub const COL_ACTIVE_WIDTH: u16 = 8;
pub const COL_LAST_UPDATED_WIDTH: u16 = 13;

// ---------------------------------------------------------------------------
// Help panel
// ---------------------------------------------------------------------------

pub const HELP_WIDTH: u16 = 44;
pub const HELP_HEIGHT: u16 = 28;

/// Display-column width for the label column in the Columns section.
/// Sized to the widest entry: "Last Updated" (12 chars) + 1 space = 13.
pub const HELP_COL_LABEL_PAD: usize = 13;

/// Display-column width for the key column in the Key bindings section.
/// Sized to the widest entry: "↵ Enter" (7 chars) + 2 spaces = 9.
pub const HELP_KEY_LABEL_PAD: usize = 9;

/// Display-column width for the description column in the State icons section.
/// Sized to the widest entry: " Awaiting input" (15 chars) + 2 spaces = 17.
pub const HELP_ICON_DESC_PAD: usize = 17;

/// Display-column width for the process-type tag column in the State icons section.
/// Sized to the widest entry: "(tc-watcher)" (12 chars) + 1 space = 13.
pub const HELP_TAG_WIDTH: usize = 13;

// ---------------------------------------------------------------------------
// Footer
// ---------------------------------------------------------------------------

pub const FOOTER_HINT: &str = "q quit  ↑↓/jk navigate  ↵ jump to pane  ? help";
pub const ERROR_TTL: Duration = Duration::from_secs(5);

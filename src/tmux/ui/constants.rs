//! Layout constants, key labels, and shared UI utilities.
//!
//! All magic numbers and repeated strings live here so a single change
//! propagates to every widget that references them.
//!
//! # Key-label strategy
//!
//! `concat!` only accepts string *literals*, not `const` variables. We
//! therefore define each key's display text as a private `macro_rules!` so
//! it can be embedded inside `concat!()` to build the hint strings below.
//! The same macros are re-exported as typed `pub const &str` for call sites
//! that need a `&str` value (e.g. the help-overlay `binding()` calls).

use ratatui::layout::Rect;
use std::time::Duration;

// ---------------------------------------------------------------------------
// Private key-label macros — one definition per key, used in concat!() below.
// ---------------------------------------------------------------------------

macro_rules! key_enter {
    () => {
        "↵"
    };
}
macro_rules! key_nav {
    () => {
        "↑↓/jk"
    };
}
macro_rules! key_esc {
    () => {
        "Esc"
    };
}
macro_rules! key_quit {
    () => {
        "q"
    };
}
macro_rules! key_new {
    () => {
        "n"
    };
}
macro_rules! key_delete {
    () => {
        "d"
    };
}
macro_rules! key_help {
    () => {
        "?"
    };
}

// ---------------------------------------------------------------------------
// Public const &str — typed re-exports for use as &str arguments elsewhere.
// Only export what is actually consumed; add more as call sites arise.
// ---------------------------------------------------------------------------

pub const KEY_QUIT: &str = key_quit!();
pub const KEY_NEW: &str = key_new!();
pub const KEY_DELETE: &str = key_delete!();

/// Enter key as shown in the help-overlay binding column ("↵ Enter").
pub const KEY_ENTER_LONG: &str = concat!(key_enter!(), " Enter");
/// Nav keys as shown in the help-overlay binding column ("↑↓ / jk").
pub const KEY_NAV_LONG: &str = "↑↓ / jk";

// ---------------------------------------------------------------------------
// Footer hint strings — fully assembled from the macros; no key string is
// ever written twice.
// ---------------------------------------------------------------------------

pub const FOOTER_HINT: &str = concat!(
    key_quit!(),
    " quit  ",
    key_nav!(),
    " nav  ",
    key_enter!(),
    " jump  ",
    key_new!(),
    " new  ",
    key_delete!(),
    " delete  ",
    key_help!(),
    " help"
);

/// Picker hint at the window level — Esc goes back one level.
pub const FOOTER_HINT_PICKER: &str = concat!(
    key_nav!(),
    " nav  ",
    key_enter!(),
    " select  ",
    key_esc!(),
    " back"
);

/// Picker hint at the session level — Esc cancels entirely.
pub const FOOTER_HINT_PICKER_TOP: &str = concat!(
    key_nav!(),
    " nav  ",
    key_enter!(),
    " select  ",
    key_esc!(),
    " cancel"
);

/// Suffix appended after the target name in a delete-confirm footer.
pub const FOOTER_CONFIRM_SUFFIX: &str =
    concat!("  ", key_enter!(), " confirm  ", key_esc!(), " cancel");

/// Close hint shown at the bottom of the help overlay.
pub const HELP_CLOSE_HINT: &str = concat!(key_help!(), " or ", key_esc!(), " to close");

// ---------------------------------------------------------------------------
// Footer TTL
// ---------------------------------------------------------------------------

pub const ERROR_TTL: Duration = Duration::from_secs(5);

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
pub const HELP_HEIGHT: u16 = 30;

/// Width of the label column in the Columns section.
pub const HELP_COL_LABEL_PAD: usize = 13;
/// Width of the key column in the Key bindings section.
pub const HELP_KEY_LABEL_PAD: usize = 9;
/// Width of the description column in the State icons section.
pub const HELP_ICON_DESC_PAD: usize = 17;
/// Width of the process-type tag column in the State icons section.
pub const HELP_TAG_WIDTH: usize = 13;

// ---------------------------------------------------------------------------
// Picker panel
// ---------------------------------------------------------------------------

pub const PICKER_WIDTH: u16 = 52;
pub const PICKER_MAX_HEIGHT: u16 = 20;

// ---------------------------------------------------------------------------
// Shared overlay geometry
// ---------------------------------------------------------------------------

/// Returns a centred [`Rect`] of the given size, clamped to `area`.
/// Shared by the help overlay and the picker overlay.
pub(super) fn centered_rect(width: u16, height: u16, area: Rect) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect {
        x,
        y,
        width: width.min(area.width),
        height: height.min(area.height),
    }
}

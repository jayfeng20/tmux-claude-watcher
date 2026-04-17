//! Catppuccin Mocha colour palette.
//! https://github.com/catppuccin/catppuccin#-palette

use ratatui::style::Color;

pub const OVERLAY0: Color = Color::Rgb(108, 112, 134); // idle / dim text
pub const OVERLAY2: Color = Color::Rgb(147, 153, 178); // awaiting input
pub const SUBTEXT0: Color = Color::Rgb(166, 173, 200); // unknown process
pub const SURFACE1: Color = Color::Rgb(69, 71, 90); // selected row background
pub const PEACH: Color = Color::Rgb(250, 179, 135); // processing / generating
pub const SKY: Color = Color::Rgb(137, 220, 235); // thinking
pub const GREEN: Color = Color::Rgb(166, 227, 161); // executing
pub const RED: Color = Color::Rgb(243, 139, 168); // error

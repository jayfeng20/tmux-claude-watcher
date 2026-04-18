//! Catppuccin Mocha colour palette.
//! https://github.com/catppuccin/catppuccin#-palette

use ratatui::style::Color;

pub const OVERLAY0: Color = Color::Rgb(108, 112, 134); // dim / inactive
pub const SUBTEXT0: Color = Color::Rgb(166, 173, 200); // unknown process
pub const SURFACE1: Color = Color::Rgb(69, 71, 90); // selected row background
pub const SAPPHIRE: Color = Color::Rgb(116, 199, 236); // awaiting permission / blocked
pub const TEAL: Color = Color::Rgb(148, 226, 213); // executing / processing
pub const PEACH: Color = Color::Rgb(250, 179, 135); // claude type label
pub const YELLOW: Color = Color::Rgb(249, 226, 175); // thinking
pub const GREEN: Color = Color::Rgb(166, 227, 161); // awaiting input / done
pub const RED: Color = Color::Rgb(243, 139, 168); // error

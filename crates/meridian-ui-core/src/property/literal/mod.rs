//! Parsing property values from markup text (`Width="Auto"`,
//! `Margin="4,2"`). Text is parsed once, here, on the way in; nothing
//! downstream sees the string.

mod error;
mod grid_length;
mod keyword;
mod length;
mod number;
mod text;
mod thickness;

pub use error::LiteralError;
pub(super) use grid_length::grid_lengths;
pub(super) use keyword::{alignment, dock, orientation, visibility};
pub(super) use length::length;
pub(super) use number::{boolean, float, index};
pub(super) use text::text;
pub(super) use thickness::thickness;

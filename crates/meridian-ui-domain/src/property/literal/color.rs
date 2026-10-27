//! `Color` literals.

use super::LiteralError;
use crate::Color;

pub(in crate::property) fn color(text: &str) -> Result<Color, LiteralError> {
    Color::parse(text)
}

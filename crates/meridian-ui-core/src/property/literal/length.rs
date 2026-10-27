//! `Length` literals: `Auto` or pixels.

use super::{LiteralError, number::float};
use crate::layout::value::Length;

pub(in crate::property) fn length(text: &str) -> Result<Length, LiteralError> {
    let trimmed = text.trim();
    if trimmed == "Auto" {
        return Ok(Length::Auto);
    }
    match float(trimmed) {
        Ok(pixels) if pixels >= 0.0 => Ok(Length::Px(pixels)),
        _ => Err(LiteralError::invalid("Auto or a size in pixels", trimmed)),
    }
}

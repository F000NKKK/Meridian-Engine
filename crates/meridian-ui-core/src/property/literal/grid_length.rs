//! Grid track definitions: `Auto`, `120`, `*`, `2*`, comma separated.

use super::LiteralError;
use super::number::float;
use crate::layout::value::GridLength;

const EXPECTED: &str = "Auto, a size in pixels, or a star weight like `*` or `2*`";

pub(super) fn grid_length(text: &str) -> Result<GridLength, LiteralError> {
    let trimmed = text.trim();
    if trimmed == "Auto" {
        return Ok(GridLength::Auto);
    }
    let invalid = || LiteralError::invalid(EXPECTED, trimmed);
    match trimmed.strip_suffix('*') {
        Some("") => Ok(GridLength::Star(1.0)),
        Some(weight) => match float(weight) {
            Ok(weight) if weight > 0.0 => Ok(GridLength::Star(weight)),
            _ => Err(invalid()),
        },
        None => match float(trimmed) {
            Ok(pixels) if pixels >= 0.0 => Ok(GridLength::Px(pixels)),
            _ => Err(invalid()),
        },
    }
}

pub(in crate::property) fn grid_lengths(text: &str) -> Result<Vec<GridLength>, LiteralError> {
    if text.trim().is_empty() {
        return Ok(Vec::new());
    }
    text.split(',').map(grid_length).collect()
}

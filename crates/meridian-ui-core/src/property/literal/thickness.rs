//! `Thickness` literals: `5`, `4,2` (horizontal, vertical) or
//! `left,top,right,bottom`.

use super::{LiteralError, number::float};
use crate::Thickness;

pub(in crate::property) fn thickness(text: &str) -> Result<Thickness, LiteralError> {
    let parts: Vec<f32> = text
        .split([',', ' '])
        .filter(|part| !part.is_empty())
        .map(float)
        .collect::<Result<_, _>>()?;
    match parts[..] {
        [all] => Ok(Thickness::uniform(all)),
        [horizontal, vertical] => Ok(Thickness::new(horizontal, vertical, horizontal, vertical)),
        [left, top, right, bottom] => Ok(Thickness::new(left, top, right, bottom)),
        _ => Err(LiteralError::invalid("1, 2 or 4 numbers", text.trim())),
    }
}

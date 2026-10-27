//! `bool`, index and float literals.

use super::LiteralError;

pub(in crate::property) fn boolean(text: &str) -> Result<bool, LiteralError> {
    match text.trim() {
        "true" | "True" => Ok(true),
        "false" | "False" => Ok(false),
        other => Err(LiteralError::invalid("true or false", other)),
    }
}

pub(in crate::property) fn index(text: &str) -> Result<u32, LiteralError> {
    text.trim()
        .parse()
        .map_err(|_| LiteralError::invalid("a whole number", text.trim()))
}

pub(in crate::property) fn float(text: &str) -> Result<f32, LiteralError> {
    let trimmed = text.trim();
    match trimmed.parse::<f32>() {
        Ok(number) if number.is_finite() => Ok(number),
        _ => Err(LiteralError::invalid("a finite number", trimmed)),
    }
}

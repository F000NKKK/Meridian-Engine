//! [`TagParseError`]: a tag that could not be built from its element.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct TagParseError {
    pub message: String,
}

impl fmt::Display for TagParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for TagParseError {}

impl meridian_diagnostics::EngineError for TagParseError {}

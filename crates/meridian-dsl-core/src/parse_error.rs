//! Why a source failed to parse.

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    /// Byte offset into the source where the error was detected — not a
    /// line/column (the source is expected to be short enough that a
    /// caller can find the offset by eye, and computing line/column
    /// would need a second pass over bytes already consumed).
    pub offset: usize,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "at byte {}: {}", self.offset, self.message)
    }
}

impl std::error::Error for ParseError {}

impl meridian_foundation::EngineError for ParseError {}

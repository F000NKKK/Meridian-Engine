//! Source positions: where in a `.meui` document an element, attribute or
//! error starts, as a byte offset plus the 1-based line and column a
//! person (or an editor) reads.

use std::fmt;

/// A position in the source text. `line` and `column` are 1-based;
/// `column` counts `char`s, not bytes, so it matches what an editor shows
/// for non-ASCII text.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Span {
    /// Zero-based byte offset in the source document.
    pub offset: usize,
    /// One-based source line.
    pub line: u32,
    /// One-based character column.
    pub column: u32,
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}", self.line, self.column)
    }
}

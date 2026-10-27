//! Why font bytes were rejected.

use thiserror::Error;

/// A font could not be read. Malformed input always ends here, never in a
/// panic or an unbounded allocation.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum FontError {
    /// The data ends in the middle of the named structure.
    #[error("font data ends inside {what}")]
    Truncated {
        /// The structure being read.
        what: &'static str,
    },
    /// A table the face cannot work without is absent.
    #[error("font table `{0}` is missing")]
    MissingTable(&'static str),
    /// The data contradicts itself.
    #[error("malformed font: {0}")]
    Malformed(&'static str),
    /// A valid font feature this version does not read.
    #[error("unsupported font feature: {0}")]
    Unsupported(&'static str),
    /// A glyph id beyond the face's glyph count.
    #[error("glyph {0} is out of range")]
    GlyphOutOfRange(u16),
}

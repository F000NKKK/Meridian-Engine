//! [`ReadAtError`]: why a [`ReadAt`](super::ReadAt) did not give bytes.

use thiserror::Error;

/// Why a [`ReadAt`](super::ReadAt) did not give bytes.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ReadAtError {
    /// The range reaches past the end of the source.
    #[error("{length} bytes at {offset} is past the end of the source ({size} bytes)")]
    OutOfRange {
        /// Where the read starts.
        offset: u64,
        /// How many bytes were asked for.
        length: u64,
        /// The size of the source.
        size: u64,
    },
    /// The source failed; `reason` is the system's text.
    #[error("read failed: {reason}")]
    Failed {
        /// What the system said.
        reason: String,
    },
}

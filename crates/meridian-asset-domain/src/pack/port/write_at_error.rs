//! [`WriteAtError`]: why a [`WriteAt`](super::WriteAt) did not take bytes.

use thiserror::Error;

/// Why a [`WriteAt`](super::WriteAt) did not take bytes.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum WriteAtError {
    /// The write would leave a gap after the end of the sink, which this
    /// sink does not allow.
    #[error(
        "{length} bytes at {offset} would leave a gap after the end of the sink ({size} bytes)"
    )]
    Gap {
        /// Where the write starts.
        offset: u64,
        /// How many bytes were given.
        length: u64,
        /// The size of the sink.
        size: u64,
    },
    /// The sink failed; `reason` is the system's text.
    #[error("write failed: {reason}")]
    Failed {
        /// What the system said.
        reason: String,
    },
}

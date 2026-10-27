//! [`FrameError`]: why a message could not be written or read.

use std::io;

use substrate_data::Diagnostic;

/// Why a message could not be written or read.
#[derive(Debug, thiserror::Error)]
pub enum FrameError {
    /// The stream failed.
    #[error("the stream failed: {0}")]
    Io(#[source] io::Error),
    /// The stream ended inside a frame.
    #[error("the stream ended inside a frame")]
    Truncated,
    /// The frame's length is over the limit.
    #[error("a frame is longer than the limit")]
    TooLong,
    /// The frame isn't a message of the expected type: bytes that don't
    /// decode, or a variant this build doesn't know (a newer peer's).
    #[error("the frame is not a known message ({} problem(s))", .0.len())]
    Malformed(Vec<Diagnostic>),
}

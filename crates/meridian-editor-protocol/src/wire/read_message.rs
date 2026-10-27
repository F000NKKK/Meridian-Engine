//! [`read_message`]: the next message off a stream.

use std::io::{ErrorKind, Read};

use substrate_data::{Schema, io};

use super::FrameError;

/// The next message; `None` where the stream ends cleanly between frames.
///
/// # Errors
///
/// [`FrameError::Truncated`] when the stream ends inside a frame,
/// [`FrameError::TooLong`] for a length over the limit,
/// [`FrameError::Malformed`] for a frame that isn't a `T` (including a
/// variant this build doesn't know), [`FrameError::Io`] when the reader fails.
pub fn read_message<T: Schema>(input: &mut impl Read) -> Result<Option<T>, FrameError> {
    let bytes = io::read_frame_bytes(input).map_err(|e| match e.kind() {
        ErrorKind::UnexpectedEof => FrameError::Truncated,
        ErrorKind::InvalidData => FrameError::TooLong,
        _ => FrameError::Io(e),
    })?;
    bytes
        .map(|bytes| substrate_data::read_bytes(&bytes).map_err(FrameError::Malformed))
        .transpose()
}

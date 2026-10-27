//! [`write_message`]: one message onto a stream.

use std::io::{ErrorKind, Write};

use substrate_data::{Schema, io};

use super::FrameError;

/// Writes `message` as one frame, and flushes.
///
/// # Errors
///
/// [`FrameError::TooLong`] for a message over the frame limit;
/// [`FrameError::Io`] when the writer fails.
pub fn write_message<T: Schema>(out: &mut impl Write, message: &T) -> Result<(), FrameError> {
    io::write_frame(out, message).map_err(|e| match e.kind() {
        ErrorKind::InvalidInput => FrameError::TooLong,
        _ => FrameError::Io(e),
    })
}

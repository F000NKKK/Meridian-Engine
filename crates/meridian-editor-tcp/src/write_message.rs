//! [`write_message`]: one message onto an async stream.

use std::io::ErrorKind;

use meridian_editor_protocol::FrameError;
use substrate_data::Schema;
use tokio::io::{AsyncWrite, AsyncWriteExt};

/// Writes `message` as one frame, and flushes. The frame is built by
/// `substrate_data::io::write_frame` and written in one piece.
///
/// # Errors
///
/// [`FrameError::TooLong`] for a message over the frame limit;
/// [`FrameError::Io`] when the writer fails.
pub async fn write_message<T, W>(out: &mut W, message: &T) -> Result<(), FrameError>
where
    T: Schema,
    W: AsyncWrite + Unpin,
{
    let mut frame = Vec::new();
    substrate_data::io::write_frame(&mut frame, message).map_err(|e| match e.kind() {
        ErrorKind::InvalidInput => FrameError::TooLong,
        _ => FrameError::Io(e),
    })?;
    out.write_all(&frame).await.map_err(FrameError::Io)?;
    out.flush().await.map_err(FrameError::Io)
}

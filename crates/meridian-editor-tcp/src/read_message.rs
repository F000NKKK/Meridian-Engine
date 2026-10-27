//! [`read_message`]: the next message off an async stream.

use std::io::ErrorKind;

use meridian_editor_domain::FrameError;
use substrate_data::Schema;
use substrate_data::io::{Len, MAX_FRAME};
use tokio::io::{AsyncRead, AsyncReadExt};

/// The next message; `None` where the stream ends cleanly between frames.
///
/// The async twin of `meridian_editor_domain::read_message`, with one
/// difference: a stream cut inside the 4-byte length is
/// [`FrameError::Truncated`] here (the synchronous `read_frame_bytes` reads
/// it as a clean end).
///
/// # Errors
///
/// [`FrameError::Truncated`] when the stream ends inside a frame,
/// [`FrameError::TooLong`] for a length over the limit,
/// [`FrameError::Malformed`] for a frame that isn't a `T`,
/// [`FrameError::Io`] when the reader fails.
pub async fn read_message<T, R>(input: &mut R) -> Result<Option<T>, FrameError>
where
    T: Schema,
    R: AsyncRead + Unpin,
{
    let Some(len) = read_len(input).await? else {
        return Ok(None);
    };
    let len = len as usize;
    if len > MAX_FRAME {
        return Err(FrameError::TooLong);
    }
    // Grows as bytes arrive: a hostile length costs nothing until it is backed.
    let mut body = Vec::new();
    let read = input
        .take(len as u64)
        .read_to_end(&mut body)
        .await
        .map_err(FrameError::Io)?;
    if read < len {
        return Err(FrameError::Truncated);
    }
    substrate_data::read_bytes(&body)
        .map(Some)
        .map_err(FrameError::Malformed)
}

/// The frame's length; `None` when the stream ends before its first byte.
async fn read_len<R: AsyncRead + Unpin>(input: &mut R) -> Result<Option<Len>, FrameError> {
    let mut len = [0; size_of::<Len>()];
    let first = input.read(&mut len[..1]).await.map_err(FrameError::Io)?;
    if first == 0 {
        return Ok(None);
    }
    match input.read_exact(&mut len[1..]).await {
        Ok(_) => Ok(Some(Len::from_le_bytes(len))),
        Err(e) if e.kind() == ErrorKind::UnexpectedEof => Err(FrameError::Truncated),
        Err(e) => Err(FrameError::Io(e)),
    }
}

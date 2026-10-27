//! [`serve_stream`]: the request/response loop over one connection.

use meridian_editor_host::{Incoming, RequestHandler, unsupported};
use substrate_data::Value;
use tokio::io::{AsyncRead, AsyncWrite, AsyncWriteExt};

use crate::{ServeError, read_message, write_message};

/// Serves one editor on `stream` until it closes its side: for each frame,
/// a request goes to `handler` and what the handler returns is written back;
/// a request this build cannot read is answered `Unsupported` with its id
/// (docs/editor-protocol.md); the stream's end, between frames, ends the
/// loop cleanly.
///
/// # Errors
///
/// [`ServeError::Frame`] when the stream fails, is cut inside a frame, or
/// carries a frame that is too long or not a message;
/// [`ServeError::BrokenPeer`] for a frame with no readable request `id`.
pub async fn serve_stream<S, H>(mut stream: S, handler: &mut H) -> Result<(), ServeError>
where
    S: AsyncRead + AsyncWrite + Unpin,
    H: RequestHandler,
{
    while let Some(frame) = read_message::<Value, _>(&mut stream).await? {
        let replies = match Incoming::classify(&frame) {
            Incoming::Request(request) => handler.handle(request),
            Incoming::Unsupported(id) => vec![unsupported(id)],
            Incoming::Unidentifiable(problems) => return Err(ServeError::BrokenPeer(problems)),
        };
        for reply in &replies {
            write_message(&mut stream, reply).await?;
        }
    }
    // The editor hung up; closing our side is best effort.
    stream.shutdown().await.ok();
    Ok(())
}

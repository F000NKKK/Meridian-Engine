//! [`ServeError`]: why serving an editor stopped.

use std::io;

use meridian_editor_protocol::FrameError;
use substrate_data::Diagnostic;

/// Why serving an editor stopped. A clean end of the stream is not an error.
#[derive(Debug, thiserror::Error)]
pub enum ServeError {
    /// Binding, or accepting a connection, failed.
    #[error("the listener failed: {0}")]
    Listen(#[source] io::Error),
    /// The stream failed, ended inside a frame, or carried a frame that is
    /// too long or not a message at all.
    #[error("the connection broke: {0}")]
    Frame(#[from] FrameError),
    /// A frame with no readable request `id`: the host cannot even say
    /// `Unsupported`, so the connection is closed.
    #[error("a frame has no readable request id ({} problem(s))", .0.len())]
    BrokenPeer(Vec<Diagnostic>),
}

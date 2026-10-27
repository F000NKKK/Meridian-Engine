//! [`EditorListener`]: the local socket an editor connects to.

use std::net::{Ipv4Addr, SocketAddr};

use meridian_editor_host::RequestHandler;
use tokio::net::TcpListener;

use crate::{ServeError, serve_stream};

/// A listening socket on `127.0.0.1`. Local only and unauthenticated: the
/// editor and the game run on one machine, and whoever can connect to the
/// port can edit the game.
#[derive(Debug)]
pub struct EditorListener {
    listener: TcpListener,
}

impl EditorListener {
    /// Binds `127.0.0.1:port`; `0` picks a free port (see [`Self::address`]).
    ///
    /// # Errors
    ///
    /// [`ServeError::Listen`] when the port cannot be bound.
    pub async fn bind(port: u16) -> Result<Self, ServeError> {
        let address = SocketAddr::from((Ipv4Addr::LOCALHOST, port));
        let listener = TcpListener::bind(address)
            .await
            .map_err(ServeError::Listen)?;
        Ok(EditorListener { listener })
    }

    /// The address bound, for the editor to connect to.
    ///
    /// # Errors
    ///
    /// [`ServeError::Listen`] when the socket cannot say.
    pub fn address(&self) -> Result<SocketAddr, ServeError> {
        self.listener.local_addr().map_err(ServeError::Listen)
    }

    /// Accepts **one** editor and serves it with `handler` until it
    /// disconnects; a second editor waits in the OS backlog until this
    /// returns (and is never served by this call).
    ///
    /// # Errors
    ///
    /// [`ServeError`]: accepting failed, or the connection broke
    /// ([`serve_stream`]).
    pub async fn serve_one<H: RequestHandler>(self, handler: &mut H) -> Result<(), ServeError> {
        let (stream, _) = self.listener.accept().await.map_err(ServeError::Listen)?;
        serve_stream(stream, handler).await
    }
}

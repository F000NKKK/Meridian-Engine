//! Serves the editor protocol to one editor over a local TCP connection.
//!
//! An [`EditorListener`] binds `127.0.0.1` (a port, or `0` for an ephemeral
//! one), accepts **one** editor and runs [`serve_stream`] on it: read a
//! frame, classify it ([`meridian_editor_app::Incoming`]), hand a request
//! to the [`meridian_editor_app::RequestHandler`], write what it returns,
//! until the editor closes the stream. [`serve_stream`] works over any
//! `AsyncRead + AsyncWrite`, so another transport (a Unix socket) is a
//! few lines. Frames are Substrate Data's (`substrate_data::io`): a
//! little-endian `u32` length, then the message; [`read_message`] and
//! [`write_message`] are the async twins of the protocol crate's.
//!
//! Tier: above `meridian-editor-app`; the one crate of the editor host
//! that depends on `tokio`, because it does real I/O. Lifecycle and what is
//! not implemented: docs/editor-protocol.md.

mod editor_listener;
mod read_message;
mod serve_error;
mod serve_stream;
mod write_message;

pub use editor_listener::EditorListener;
pub use read_message::read_message;
pub use serve_error::ServeError;
pub use serve_stream::serve_stream;
pub use write_message::write_message;

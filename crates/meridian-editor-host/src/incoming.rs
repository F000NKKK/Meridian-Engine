//! [`Incoming`]: a frame from the editor, classified.

use meridian_editor_protocol::{HostMessage, RequestEnvelope, RequestError, RequestId, Response};
use substrate_data::{Diagnostic, Value};

/// What a decoded frame turned out to be. The wire form refuses a request
/// it does not know (a newer editor's), and then the host has no
/// [`RequestEnvelope`]; this is the documented way to still answer it
/// (docs/editor-protocol.md, "Requests the host cannot read").
#[derive(Debug, Clone, PartialEq)]
pub enum Incoming {
    /// A request this build understands.
    Request(RequestEnvelope),
    /// A frame with a readable `id` but a request this build does not know:
    /// answer it with [`unsupported`].
    Unsupported(RequestId),
    /// A frame with no readable `id`: a broken peer, to disconnect.
    Unidentifiable(Vec<Diagnostic>),
}

impl Incoming {
    /// Classifies `frame`, any frame read as a [`Value`]: take its `id`,
    /// then read the whole as a [`RequestEnvelope`].
    #[must_use]
    pub fn classify(frame: &Value) -> Self {
        match substrate_data::read_value::<RequestEnvelope>(frame) {
            Ok(envelope) => Incoming::Request(envelope),
            Err(problems) => match frame.get("id").map(substrate_data::read_value::<RequestId>) {
                Some(Ok(id)) => Incoming::Unsupported(id),
                _ => Incoming::Unidentifiable(problems),
            },
        }
    }
}

/// The answer to a request this host cannot read: `Failed { Unsupported }`
/// with the request's id.
#[must_use]
pub fn unsupported(id: RequestId) -> HostMessage {
    HostMessage::Response {
        id,
        response: Response::Failed {
            error: RequestError::Unsupported,
        },
    }
}

//! [`RequestHandler`]: the port a transport calls with each request.

use meridian_editor_domain::{HostMessage, RequestEnvelope};

/// Whatever answers an editor's requests: a transport (`meridian-editor-tcp`)
/// reads a frame, decodes it, and hands the request here. The answer is
/// the messages to send back, in order. Synchronous on purpose: answering
/// touches the `World`, which the host owns.
pub trait RequestHandler {
    /// The messages to send for `request`: its response, then any events.
    fn handle(&mut self, request: RequestEnvelope) -> Vec<HostMessage>;
}

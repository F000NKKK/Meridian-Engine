//! [`HostMessage`]: everything the host sends the editor.

use substrate_data::Schema;

use crate::{Event, RequestId, Response};

/// One message from the host to the editor: the answer to a request, or an
/// event the editor subscribed to. Variants are only ever appended.
#[derive(Debug, Clone, PartialEq, Schema)]
pub enum HostMessage {
    /// The answer to the request with this id.
    Response {
        /// The request's id.
        id: RequestId,
        /// The answer.
        response: Response,
    },
    /// An event.
    Event {
        /// What happened.
        event: Event,
    },
}

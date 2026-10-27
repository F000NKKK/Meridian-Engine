//! [`RequestEnvelope`]: a request and the id its answer will carry.

use substrate_data::Schema;

use crate::{Request, RequestId};

/// One message from the editor to the host. Fields are only ever appended.
#[derive(Debug, Clone, PartialEq, Schema)]
pub struct RequestEnvelope {
    /// Echoed in the response.
    pub id: RequestId,
    /// What is asked.
    pub request: Request,
}

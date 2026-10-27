//! [`RequestId`]: pairs a response with its request.

use substrate_data::Schema;

/// Chosen by the editor per request and echoed in the response to it, so
/// answers may arrive in any order among events. Unique among the
/// requests in flight on one connection; otherwise the editor's business.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Schema)]
pub struct RequestId(u64);

impl RequestId {
    /// `number` as a request id.
    #[must_use]
    pub const fn new(number: u64) -> Self {
        RequestId(number)
    }

    /// The id's number.
    #[must_use]
    pub const fn number(self) -> u64 {
        self.0
    }

    /// The id after this one.
    #[must_use]
    pub const fn next(self) -> Self {
        RequestId(self.0.wrapping_add(1))
    }
}

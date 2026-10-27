//! [`ProtocolVersion`]: which revision of the protocol a peer speaks.

use substrate_data::Schema;

use crate::RequestError;

/// The revision of the protocol a peer speaks. Messages are append-only, so
/// a host understands every client whose version is not newer than its own;
/// the version is bumped when messages are added (or, rarely, when one
/// must change meaning).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Schema)]
pub struct ProtocolVersion(u32);

impl ProtocolVersion {
    /// The version this crate speaks.
    pub const CURRENT: ProtocolVersion = ProtocolVersion(1);

    /// `number` as a version.
    #[must_use]
    pub const fn new(number: u32) -> Self {
        ProtocolVersion(number)
    }

    /// The version's number.
    #[must_use]
    pub const fn number(self) -> u32 {
        self.0
    }

    /// Whether a host speaking `self` understands a client speaking `client`.
    ///
    /// # Errors
    ///
    /// [`RequestError::VersionMismatch`] when the client is newer than the host.
    pub fn accepts(self, client: ProtocolVersion) -> Result<(), RequestError> {
        if client <= self {
            Ok(())
        } else {
            Err(RequestError::VersionMismatch { host: self, client })
        }
    }
}

//! The port that supplies an asset's bytes.

use crate::{AssetPath, SourceError};

/// Reads the bytes of an asset. Blocking: it may touch a disk or a network,
/// so it runs on a background job, never on the frame thread. `Send + Sync`
/// because every load shares one source.
pub trait ByteSource: Send + Sync {
    /// The whole content of `path`.
    ///
    /// # Errors
    ///
    /// [`SourceError::NotFound`] if there is no such asset, and
    /// [`SourceError::Rejected`] / [`SourceError::Unreadable`] otherwise.
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>, SourceError>;
}

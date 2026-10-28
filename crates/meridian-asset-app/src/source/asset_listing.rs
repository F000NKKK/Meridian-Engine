//! The port that says which assets a source holds.

use crate::{AssetPath, ByteSource, SourceError};

/// A [`ByteSource`] that can also name everything it holds. Not every
/// source can (a web server has no directory to walk), so this is a
/// separate port: a catalog scan needs it, a plain load does not.
pub trait AssetListing: ByteSource {
    /// Every asset path in the source, sorted, `/`-separated. Blocking, like
    /// [`ByteSource::read`].
    ///
    /// # Errors
    ///
    /// [`SourceError::Unreadable`] when the source cannot be walked.
    fn paths(&self) -> Result<Vec<AssetPath>, SourceError>;
}

//! An asset pack as a [`ByteSource`].

use meridian_asset_domain::{PackError, PackPath, PackReader, ReadAt};

use crate::{AssetPath, ByteSource, SourceError};

/// A [`ByteSource`] that serves the assets of a `.mpak`: the asset path is
/// the path inside the pack, and the bytes are checked against the pack's
/// checksum before they are handed out. Everything above it (the
/// [`AssetLoader`](crate::AssetLoader), prefab and scene loading) works on a
/// pack exactly as on a directory.
#[derive(Debug)]
pub struct PackSource<R> {
    reader: PackReader<R>,
}

impl<R: ReadAt> PackSource<R> {
    /// Opens the pack in `source`: reads its header and index, not its assets.
    ///
    /// # Errors
    ///
    /// [`PackError`] when `source` is not a readable pack.
    pub fn open(source: R) -> Result<Self, PackError> {
        PackReader::open(source).map(|reader| PackSource { reader })
    }

    /// The pack, for its entries and for reading by id.
    #[must_use]
    pub fn reader(&self) -> &PackReader<R> {
        &self.reader
    }
}

impl<R: ReadAt + Send + Sync> ByteSource for PackSource<R> {
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>, SourceError> {
        let in_pack =
            PackPath::new(path.as_str()).map_err(|_| SourceError::Rejected(path.clone()))?;
        self.reader
            .read_path(&in_pack)
            .map_err(|error| match error {
                PackError::NoSuchPath(_) => SourceError::NotFound(path.clone()),
                other => SourceError::Unreadable {
                    path: path.clone(),
                    reason: other.to_string(),
                },
            })
    }
}

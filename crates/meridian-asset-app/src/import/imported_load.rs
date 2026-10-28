//! Loading an asset by what its `.meta` says.

use std::sync::Arc;

use super::{ImportError, ImporterRegistry, MetaReadError, read_meta};
use crate::{AssetLoader, AssetPath, LoadError, Pending, Priority};

impl AssetLoader {
    /// Reads `path`'s `.meta`, picks the importer it names from `importers`
    /// and imports the asset, all on one background job at
    /// [`Priority::Normal`]. A missing sidecar is
    /// `LoadError::Source(NotFound(<path>.meta))`, a bad one
    /// `LoadError::Decode(ImportError::InvalidMeta)`, an unregistered name
    /// `ImportError::UnknownImporter`.
    #[must_use]
    pub fn load_imported<T: Send + 'static>(
        &self,
        path: AssetPath,
        importers: Arc<ImporterRegistry<T>>,
    ) -> Pending<Result<T, LoadError<ImportError>>> {
        self.load_imported_with(Priority::Normal, path, importers)
    }

    /// [`load_imported`](Self::load_imported) with an explicit `priority`.
    #[must_use]
    pub fn load_imported_with<T: Send + 'static>(
        &self,
        priority: Priority,
        path: AssetPath,
        importers: Arc<ImporterRegistry<T>>,
    ) -> Pending<Result<T, LoadError<ImportError>>> {
        self.submit(priority, move |source| {
            let meta = read_meta(source, &path).map_err(|error| match error {
                MetaReadError::Source(error) => LoadError::Source(error),
                MetaReadError::Invalid {
                    path: meta_path,
                    error,
                } => LoadError::Decode(ImportError::InvalidMeta {
                    path: meta_path,
                    error,
                }),
            })?;
            let bytes = source.read(&path).map_err(LoadError::Source)?;
            importers.import(&meta, &bytes).map_err(LoadError::Decode)
        })
    }
}

//! [`ImporterRegistry`]: importer names to importers.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use meridian_asset_domain::{AssetMeta, Decoder, ImporterName};

use super::{FromDecoder, ImportError, Importer, RegisterError};

/// The importers for content of type `T`, by the name a `.meta` file gives.
///
/// Owned by whoever builds it (the composition root registers the importers
/// the application ships), never global; one registry per content type, so
/// an `obj` importer for meshes and a `png` importer for images are in
/// different registries and a name cannot return the wrong type. Cheap to
/// share: wrap it in an `Arc`. It holds code, not content: what an importer
/// returns is the caller's to keep or drop.
pub struct ImporterRegistry<T> {
    importers: HashMap<ImporterName, Arc<dyn Importer<T>>>,
}

impl<T> ImporterRegistry<T> {
    /// A registry with no importer.
    #[must_use]
    pub fn new() -> Self {
        ImporterRegistry {
            importers: HashMap::new(),
        }
    }

    /// Registers `importer` under `name`.
    ///
    /// # Errors
    ///
    /// [`RegisterError::NameTaken`] when `name` already has one.
    pub fn register(
        &mut self,
        name: ImporterName,
        importer: impl Importer<T> + 'static,
    ) -> Result<(), RegisterError> {
        if self.importers.contains_key(&name) {
            return Err(RegisterError::NameTaken(name));
        }
        self.importers.insert(name, Arc::new(importer));
        Ok(())
    }

    /// Registers a plain `decoder` under `name`; its settings are not read.
    ///
    /// # Errors
    ///
    /// [`RegisterError::NameTaken`] when `name` already has one.
    pub fn register_decoder<D>(
        &mut self,
        name: ImporterName,
        decoder: D,
    ) -> Result<(), RegisterError>
    where
        D: Decoder<T> + Send + Sync + 'static,
        D::Error: std::error::Error + Send + Sync + 'static,
    {
        self.register(name, FromDecoder(decoder))
    }

    /// The importer named `name`.
    #[must_use]
    pub fn get(&self, name: &ImporterName) -> Option<&dyn Importer<T>> {
        self.importers.get(name).map(|importer| &**importer)
    }

    /// The registered names, sorted.
    #[must_use]
    pub fn names(&self) -> Vec<&ImporterName> {
        let mut names: Vec<_> = self.importers.keys().collect();
        names.sort();
        names
    }

    /// Imports `bytes` with the importer `meta` names.
    ///
    /// # Errors
    ///
    /// [`ImportError::UnknownImporter`] when the registry has no such name,
    /// [`ImportError::Failed`] when the importer rejects the bytes.
    pub fn import(&self, meta: &AssetMeta, bytes: &[u8]) -> Result<T, ImportError> {
        let importer = self
            .get(&meta.importer)
            .ok_or_else(|| ImportError::UnknownImporter(meta.importer.clone()))?;
        importer
            .import(bytes, meta)
            .map_err(|source| ImportError::Failed {
                importer: meta.importer.clone(),
                source,
            })
    }
}

impl<T> Default for ImporterRegistry<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> fmt::Debug for ImporterRegistry<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_set().entries(self.names()).finish()
    }
}

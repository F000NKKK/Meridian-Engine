//! [`AssetCatalog`]: the `.meta` files of a source, read.

use std::collections::HashMap;

use meridian_asset_domain::AssetId;

use super::cycle::find_cycle;
use super::{CatalogEntry, CatalogError};
use crate::{AssetListing, AssetPath, read_meta};

/// The suffix of a sidecar path.
const SIDECAR: &str = ".meta";

/// Every asset of a source that has a `.meta`, found by walking the source
/// and reading each sidecar: id to entry and path to id. Built once at
/// load time (blocking: off the frame thread), then read-only. It is the
/// map from names on disk to stable ids that the `resource` id table
/// starts from; it loads nothing, owns no content and decides no lifetime.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetCatalog {
    by_id: HashMap<AssetId, CatalogEntry>,
    by_path: HashMap<AssetPath, AssetId>,
    order: Vec<AssetId>,
}

impl AssetCatalog {
    /// Reads every `*.meta` that `source` lists. A source with none gives
    /// an empty catalog.
    ///
    /// # Errors
    ///
    /// [`CatalogError`] when the source cannot be walked, a `.meta` does not
    /// read, two assets share an id, a dependency names an id no `.meta`
    /// declares, or dependencies form a loop. Nothing partial is returned.
    pub fn scan(source: &dyn AssetListing) -> Result<Self, CatalogError> {
        let mut sidecars: Vec<AssetPath> = source
            .paths()
            .map_err(CatalogError::Listing)?
            .into_iter()
            .filter(|path| path.as_str().ends_with(SIDECAR))
            .collect();
        sidecars.sort_by(|a, b| a.as_str().cmp(b.as_str()));
        let mut catalog = AssetCatalog {
            by_id: HashMap::new(),
            by_path: HashMap::new(),
            order: Vec::new(),
        };
        for sidecar in sidecars {
            catalog.add(source, &sidecar)?;
        }
        catalog.check_dependencies()?;
        Ok(catalog)
    }

    fn add(&mut self, source: &dyn AssetListing, sidecar: &AssetPath) -> Result<(), CatalogError> {
        let text = sidecar.as_str();
        let asset = AssetPath::new(&text[..text.len() - SIDECAR.len()]);
        let meta = read_meta(source, &asset)?;
        if let Some(first) = self.by_id.get(&meta.id) {
            return Err(CatalogError::DuplicateId {
                id: meta.id,
                first: first.path.clone(),
                second: asset,
            });
        }
        self.by_path.insert(asset.clone(), meta.id);
        self.order.push(meta.id);
        self.by_id.insert(meta.id, CatalogEntry::of(asset, meta));
        Ok(())
    }

    fn check_dependencies(&self) -> Result<(), CatalogError> {
        for entry in self.iter() {
            if let Some(&dependency) = entry
                .dependencies
                .iter()
                .find(|dependency| !self.by_id.contains_key(dependency))
            {
                return Err(CatalogError::MissingDependency {
                    asset: entry.path.clone(),
                    dependency,
                });
            }
        }
        match find_cycle(&self.by_id) {
            Some(id) => Err(CatalogError::DependencyCycle {
                asset: self.by_id[&id].path.clone(),
            }),
            None => Ok(()),
        }
    }

    /// The asset with this id.
    #[must_use]
    pub fn get(&self, id: AssetId) -> Option<&CatalogEntry> {
        self.by_id.get(&id)
    }

    /// The id of the asset at `path`.
    #[must_use]
    pub fn id_of(&self, path: &AssetPath) -> Option<AssetId> {
        self.by_path.get(path).copied()
    }

    /// The asset at `path`.
    #[must_use]
    pub fn by_path(&self, path: &AssetPath) -> Option<&CatalogEntry> {
        self.id_of(path).and_then(|id| self.get(id))
    }

    /// The entries, in path order.
    pub fn iter(&self) -> impl Iterator<Item = &CatalogEntry> {
        self.order.iter().map(|id| &self.by_id[id])
    }

    /// How many assets.
    #[must_use]
    pub fn len(&self) -> usize {
        self.by_id.len()
    }

    /// Whether no asset has a `.meta`.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.by_id.is_empty()
    }
}

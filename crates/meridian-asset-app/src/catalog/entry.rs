//! [`CatalogEntry`]: what a `.meta` says about one asset, with where it is.

use meridian_asset_domain::{AssetId, AssetMeta, ImporterName};

use crate::AssetPath;

/// One asset of a catalog: its stable id, the path of its source file in
/// the source it was found in, which importer reads it and what it needs.
/// Identity on disk only: no handle, no lifetime, nothing loaded.
#[derive(Debug, Clone, PartialEq)]
pub struct CatalogEntry {
    /// The asset's stable identity.
    pub id: AssetId,
    /// The asset's own path (its `.meta` path without `.meta`).
    pub path: AssetPath,
    /// Which importer reads it.
    pub importer: ImporterName,
    /// The assets it needs, all of them in the catalog.
    pub dependencies: Vec<AssetId>,
}

impl CatalogEntry {
    pub(super) fn of(path: AssetPath, meta: AssetMeta) -> Self {
        CatalogEntry {
            id: meta.id,
            path,
            importer: meta.importer,
            dependencies: meta.dependencies,
        }
    }
}

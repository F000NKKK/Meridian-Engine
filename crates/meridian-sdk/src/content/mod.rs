//! Content as the application sees it: which importers exist and where
//! the assets come from (a directory, packs, or both).

mod importers;

pub use importers::{Importers, ImportersError};
// The asset layer's catalog and importer types, and the identity types an
// application binds loaded content to, so it needs no other dependency.
pub use meridian_asset_app::{
    AssetCatalog, AssetListing, CatalogEntry, CatalogError, FromDecoder, ImportError,
    ImportFailure, Importer, ImporterRegistry, MetaReadError, RegisterError, read_meta,
};
pub use meridian_asset_domain::{AssetId, AssetMeta, ImporterName};
pub use meridian_memory::{Handle, ResourcePool};
pub use meridian_resource::{BindError, Binding, ResourceId, ResourceIdTable};

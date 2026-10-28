//! Content as the application sees it: which importers exist and where
//! the assets come from (a directory, packs, or both).

mod importers;
mod layer;
mod layers;
mod mounted;
mod open_error;

pub use importers::{Importers, ImportersError};
pub use layer::Layer;
pub use layers::Layers;
// The asset layer's catalog and importer types, and the identity types an
// application binds loaded content to, so it needs no other dependency.
pub use meridian_asset_app::{
    AssetCatalog, AssetListing, CatalogEntry, CatalogError, FromDecoder, ImportError,
    ImportFailure, Importer, ImporterRegistry, MetaReadError, OverlaySource, PackSource,
    RegisterError, read_meta,
};
pub use meridian_asset_domain::{AssetId, AssetMeta, ImporterName};
pub use meridian_memory::{Handle, ResourcePool};
pub use meridian_resource::{BindError, Binding, ResourceId, ResourceIdTable};
pub use mounted::Mounted;
pub use open_error::OpenError;

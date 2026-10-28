//! The asset catalog: every asset a source holds, by the identity its
//! `.meta` gives it.

mod asset_catalog;
mod cycle;
mod entry;
mod error;
#[cfg(test)]
mod tests;

pub use asset_catalog::AssetCatalog;
pub use entry::CatalogEntry;
pub use error::CatalogError;

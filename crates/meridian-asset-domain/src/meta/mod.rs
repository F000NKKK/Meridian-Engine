//! The `.meta` sidecar: an asset's stable id, importer, import settings
//! and dependencies (docs/data-formats.md, "Asset metadata").

mod asset_meta;
mod document;
mod error;
mod format;
mod importer_name;
mod read;
#[cfg(test)]
mod tests;

pub use asset_meta::AssetMeta;
pub use error::MetaError;
pub use format::META_FORMAT;
pub use importer_name::{ImporterName, ImporterNameError};

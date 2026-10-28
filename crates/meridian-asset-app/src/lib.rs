//! Asset loading ports and the background loader.
//!
//! Layer: application; bounded context: assets (`craftbook.toml`,
//! docs/context-map.md). `asset-domain` decodes bytes that are already in
//! memory; this crate gets the bytes there without blocking the frame:
//! [`AssetLoader`] reads from a [`ByteSource`] and decodes on a [`Spawn`]ed
//! job, and the caller polls or waits on the returned [`Pending`] (ADR 027).
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod asset_listing;
mod asset_loader;
mod asset_path;
mod byte_source;
mod catalog;
mod import;
mod load_error;
#[cfg(test)]
mod memory_source;
mod pack_source;
#[cfg(test)]
mod pack_source_tests;
mod pending;
mod priority;
mod source_error;
mod spawn;
#[cfg(test)]
mod tests;

pub use asset_listing::AssetListing;
pub use asset_loader::AssetLoader;
pub use asset_path::AssetPath;
pub use byte_source::ByteSource;
pub use catalog::{AssetCatalog, CatalogEntry, CatalogError};
pub use import::{
    FromDecoder, ImportError, ImportFailure, Importer, ImporterRegistry, MetaReadError,
    RegisterError, read_meta,
};
pub use load_error::LoadError;
pub use pack_source::PackSource;
pub use pending::Pending;
pub use priority::Priority;
pub use source_error::SourceError;
pub use spawn::Spawn;

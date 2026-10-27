//! Asset loading ports and the background loader.
//!
//! Layer: application; bounded context: assets (`craftbook.toml`,
//! docs/context-map.md). `asset-domain` decodes bytes that are already in
//! memory; this crate gets the bytes there without blocking the frame:
//! [`AssetLoader`] reads from a [`ByteSource`] and decodes on a [`Spawn`]ed
//! job, and the caller polls or waits on the returned [`Pending`] (ADR 027).
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

mod asset_loader;
mod asset_path;
mod byte_source;
mod load_error;
mod pending;
mod source_error;
mod spawn;
#[cfg(test)]
mod tests;

pub use asset_loader::AssetLoader;
pub use asset_path::AssetPath;
pub use byte_source::ByteSource;
pub use load_error::LoadError;
pub use pending::Pending;
pub use source_error::SourceError;
pub use spawn::Spawn;

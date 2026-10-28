//! Where asset bytes come from: the source ports and the sources that need
//! no adapter (a pack, a stack of sources).

mod asset_listing;
mod byte_source;
mod error;
#[cfg(test)]
mod memory_source;
mod overlay_source;
#[cfg(test)]
mod overlay_source_tests;
mod pack_source;
#[cfg(test)]
mod pack_source_tests;

pub use asset_listing::AssetListing;
pub use byte_source::ByteSource;
pub use error::SourceError;
#[cfg(test)]
pub(crate) use memory_source::MemorySource;
pub use overlay_source::OverlaySource;
pub use pack_source::PackSource;

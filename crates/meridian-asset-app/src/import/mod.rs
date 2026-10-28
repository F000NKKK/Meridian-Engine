//! Importers: what a `.meta` file's `importer` name resolves to.
//!
//! An [`Importer`] turns an asset's bytes (and its `.meta` settings) into
//! decoded content; an [`ImporterRegistry`] is the table from
//! [`ImporterName`](meridian_asset_domain::ImporterName) to importer that the
//! application builds at startup and passes where it is needed. There is no
//! global registry: the composition root owns it (docs/data-formats.md).

mod error;
mod from_decoder;
mod imported_load;
mod importer;
mod importer_registry;
mod meta_read_error;
mod read_meta;
mod register_error;
#[cfg(test)]
mod tests;

pub use error::ImportError;
pub use from_decoder::FromDecoder;
pub use importer::{ImportFailure, Importer};
pub use importer_registry::ImporterRegistry;
pub use meta_read_error::MetaReadError;
pub use read_meta::read_meta;
pub use register_error::RegisterError;

//! The `.mmat` material file: PBR parameters and texture references
//! (docs/data-formats.md, "Materials").

mod asset;
mod blend;
mod decoder;
mod document;
mod error;
mod format;
mod read;
#[cfg(test)]
mod tests;

pub use asset::MaterialAsset;
pub use blend::MaterialBlend;
pub use decoder::MaterialDecoder;
pub use error::MaterialError;
pub use format::MATERIAL_FORMAT;

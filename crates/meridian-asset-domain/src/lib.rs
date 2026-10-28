//! Loading and decoding of images, meshes, audio, shaders and binary formats. Does not manage ownership or lifetime of loaded data.
//!
//! Layer: domain; bounded context: assets (`craftbook.toml`,
//! docs/context-map.md).
//!
//! This crate must never define an `AssetManager`, `ResourceManager`, or
//! `CacheManager` type — see `docs/dependency-rules.md` rule 4.
//!
//! Real decoders, not stubs, and none needs an external codec library:
//! hand-rolled BMP, Radiance HDR and WAV, a minimal OBJ and UTF-8 shader source. They
//! are pure parsers over in-memory bytes, so they stay in the domain.
//! Formats that need a codec library are adapters implementing
//! [`Decoder`]: `meridian-asset-png` (PNG) and `meridian-asset-symphonia`
//! (MP3, OGG/Vorbis, FLAC, OGG/Opus, and streaming), per docs/adr/024.
//! Meridian's own asset formats are here too, on Substrate Data and with
//! typed errors: [`AssetMeta`] (the `.meta` sidecar: stable [`AssetId`],
//! importer, settings, dependencies) and the `.mpak` pack ([`PackWriter`],
//! [`PackReader`] over the [`ReadAt`] port, [`upgrade_pack`]); see
//! docs/data-formats.md.
//! Formats are identified by leading magic bytes ([`AudioFormat::detect`],
//! [`ImageFormat::detect`]), never by file extension.

#![forbid(unsafe_code)]

mod asset_id;
pub mod audio;
mod audio_clip;
mod bytes;
mod decode_error;
mod decoder;
mod hdr_image_asset;
pub mod image;
mod image_asset;
mod mesh_asset;
mod meta;
mod model;
mod obj_decoder;
mod pack;
mod shader_source;
mod shader_source_decoder;
#[cfg(test)]
mod tests;

pub use asset_id::{AssetId, AssetIdError, AssetKind};
pub use audio::{AudioFormat, WavDecoder};
pub use audio_clip::AudioClip;
pub(crate) use bytes::{need, u16_le, u32_le};
pub use decode_error::DecodeError;
pub use decoder::Decoder;
pub use hdr_image_asset::HdrImageAsset;
pub use image::{BmpDecoder, HdrDecoder, ImageFormat};
pub use image_asset::ImageAsset;
pub use mesh_asset::MeshAsset;
pub use meta::{AssetMeta, ImporterName, ImporterNameError, META_FORMAT, MetaError};
pub use model::{
    EncodedImage, ModelAsset, ModelChannel, ModelClip, ModelInterpolation, ModelJoint,
    ModelMaterial, ModelPart, ModelProperty, ModelSkeleton, PartSkin,
};
pub use obj_decoder::ObjDecoder;
pub use pack::{
    MAX_INDEX_LEN, PACK_MAGIC, PACK_VERSION, PackBuildError, PackEntry, PackError, PackPath,
    PackPathError, PackReader, PackWriter, ReadAt, ReadAtError, upgrade_pack,
};
pub use shader_source::ShaderSource;
pub use shader_source_decoder::ShaderSourceDecoder;

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
//! Formats are identified by leading magic bytes ([`AudioFormat::detect`],
//! [`ImageFormat::detect`]), never by file extension.

#![forbid(unsafe_code)]

pub mod audio;
mod audio_clip;
mod bytes;
mod decode_error;
mod decoder;
mod hdr_image_asset;
pub mod image;
mod image_asset;
mod mesh_asset;
mod model;
mod obj_decoder;
mod shader_source;
mod shader_source_decoder;
#[cfg(test)]
mod tests;

pub use audio::{AudioFormat, WavDecoder};
pub use audio_clip::AudioClip;
pub(crate) use bytes::{need, u16_le, u32_le};
pub use decode_error::DecodeError;
pub use decoder::Decoder;
pub use hdr_image_asset::HdrImageAsset;
pub use image::{BmpDecoder, HdrDecoder, ImageFormat};
pub use image_asset::ImageAsset;
pub use mesh_asset::MeshAsset;
pub use model::{EncodedImage, ModelAsset, ModelMaterial, ModelPart};
pub use obj_decoder::ObjDecoder;
pub use shader_source::ShaderSource;
pub use shader_source_decoder::ShaderSourceDecoder;

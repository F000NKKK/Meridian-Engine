//! Loading and decoding of images, meshes, audio, shaders and binary formats. Does not manage ownership or lifetime of loaded data.
//!
//! Layer: domain; bounded context: assets (`craftbook.toml`,
//! docs/context-map.md).
//!
//! This crate must never define an `AssetManager`, `ResourceManager`, or
//! `CacheManager` type — see `docs/dependency-rules.md` rule 4.
//!
//! Real decoders, not stubs. Images live in the [`image`] barrel module
//! (hand-rolled BMP, PNG via an external crate — see that module's own
//! doc for why); meshes get a hand-rolled minimal OBJ decoder directly
//! here (no format complex enough yet to need its own module). Audio
//! decoding lives in the [`audio`] barrel module — [`open_audio`] is its
//! thin, configuration-driven loading interface (full decode vs.
//! streaming per [`DecodeStrategy`]); compressed formats (MP3,
//! OGG/Vorbis, FLAC, OGG/Opus) use external codec crates per
//! docs/adr/013-compressed-audio-codecs.md and are identified by leading
//! magic bytes ([`AudioFormat::detect`]), never by file extension — the
//! same signature-not-extension rule [`image`]'s decoders follow. JPEG
//! and glTF remain future work on the same
//! when-a-concrete-asset-needs-it trigger.

#![forbid(unsafe_code)]

pub mod audio;
mod audio_clip;
mod bytes;
mod decode_error;
mod decoder;
pub mod image;
mod image_asset;
mod mesh_asset;
mod obj_decoder;
mod shader_source;
mod shader_source_decoder;
#[cfg(test)]
mod tests;

pub use audio::{
    AnyAudioDecoder, AudioAsset, AudioFormat, DecodeMode, DecodeStrategy, FlacDecoder, Mp3Decoder,
    OpusDecoder, StreamingAudioDecoder, VorbisDecoder, WavDecoder, open_audio,
};
pub use audio_clip::AudioClip;
pub(crate) use bytes::{need, u16_le, u32_le};
pub use decode_error::DecodeError;
pub use decoder::Decoder;
pub use image::{AnyImageDecoder, BmpDecoder, ImageFormat, PngDecoder};
pub use image_asset::ImageAsset;
pub use mesh_asset::MeshAsset;
pub use obj_decoder::ObjDecoder;
pub use shader_source::ShaderSource;
pub use shader_source_decoder::ShaderSourceDecoder;

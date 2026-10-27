//! Compressed-audio adapter for `meridian-asset-domain`: MP3, OGG/Vorbis, FLAC and
//! OGG/Opus on `symphonia` and `libopus`.
//!
//! Layer: infrastructure; bounded context: assets (`craftbook.toml`,
//! docs/context-map.md). The per-format decoders implement the `Decoder`
//! port of the domain; [`open_audio`] adds incremental decoding (ADR 013,
//! ADR 014, ADR 024).

#![forbid(unsafe_code)]

mod any_audio_decoder;
mod codec_registry;
mod decode_compressed;
mod flac_decoder;
mod mp3_decoder;
mod opus_decoder;
pub mod streaming;
#[cfg(test)]
mod tests;
mod vorbis_decoder;

pub use any_audio_decoder::AnyAudioDecoder;
pub use flac_decoder::FlacDecoder;
pub use mp3_decoder::Mp3Decoder;
pub use opus_decoder::OpusDecoder;
pub use streaming::{AudioAsset, DecodeMode, DecodeStrategy, StreamingAudioDecoder, open_audio};
pub use vorbis_decoder::VorbisDecoder;

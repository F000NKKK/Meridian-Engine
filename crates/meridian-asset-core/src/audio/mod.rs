//! Audio asset decoding, one file per concern (barrel module):
//!
//! - [`wav`]: hand-rolled PCM WAV.
//! - [`compressed_audio`]: MP3/OGG-Vorbis/FLAC/Opus via the shared
//!   symphonia + libopus pipeline (ADR 013), identified by leading magic
//!   bytes, never by file extension.
//! - [`audio_streaming`]: incremental block decoding (ADR 014).
//!
//! [`open_audio`] is *the* loading interface: a thin front door over
//! both full decoding and streaming, resolved entirely by the
//! [`DecodeStrategy`] configuration (force either path, or let `Auto`
//! pick from the estimated decoded size). Callers that don't care about
//! the split never touch the per-format decoders directly.
//!
//! A future video pipeline gets its own sibling module (`video/`) with
//! its own frame-oriented streaming shape — audio blocks and video
//! frames are different enough that no shared "asset stream" trait is
//! invented ahead of that second concrete case; the standard pull
//! abstraction both will share is `Iterator` (see ADR 014).

pub mod audio_streaming;
pub mod compressed_audio;
pub mod wav;

pub use audio_streaming::{
    AudioAsset, DecodeMode, DecodeStrategy, StreamingAudioDecoder, open_audio,
};
pub use compressed_audio::{
    AnyAudioDecoder, AudioFormat, FlacDecoder, Mp3Decoder, OpusDecoder, VorbisDecoder,
};
pub use wav::WavDecoder;

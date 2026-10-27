//! Audio asset decoding that needs no codec library:
//!
//! - `format`: format detection from leading magic bytes, never
//!   from a file extension.
//! - [`wav_decoder`]: hand-rolled PCM WAV.
//!
//! Compressed formats (MP3, OGG/Vorbis, FLAC, OGG/Opus) and incremental
//! (streaming) decoding need an external codec library and live in the
//! adapter crate `meridian-asset-symphonia` (ADR 013, ADR 014, ADR 024).

pub mod format;
pub mod wav_decoder;

pub use format::AudioFormat;
pub use wav_decoder::WavDecoder;

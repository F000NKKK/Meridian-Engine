//! Incremental ("streaming") audio decoding — see
//! docs/adr/014-streaming-audio-decode.md.
//!
//! Eager decoding ([`AnyAudioDecoder`](crate::AnyAudioDecoder)) holds
//! the *entire* decoded PCM in memory — roughly 10× the compressed
//! size; a few minutes of music is tens of megabytes. Streaming keeps
//! the compressed bytes and decodes one bounded block at a time, so
//! memory stays at compressed-size + one block regardless of track
//! length. This is incremental *decode*, not incremental I/O: the
//! compressed bytes still arrive complete, per this crate's
//! bytes-in/data-out contract (no filesystem, no lifetime policy).
//!
//! [`DecodeStrategy`] is the tunable front door: force either mode, or
//! let `Auto` pick by comparing the *estimated decoded size* (from
//! container metadata) against a configurable threshold — with
//! unknown-length tracks streaming by definition (an unbounded decode
//! can't be safely eager). The block size is the "buffer" knob.

mod audio_asset;
mod audio_decoder;
mod decode_mode;
mod decode_strategy;
mod open_audio;
mod stream_backend;
mod symphonia_track;
#[cfg(test)]
mod tests;

pub use audio_asset::AudioAsset;
pub use audio_decoder::StreamingAudioDecoder;
pub use decode_mode::DecodeMode;
pub use decode_strategy::DecodeStrategy;
pub use open_audio::open_audio;
pub(crate) use stream_backend::StreamBackend;
pub(crate) use symphonia_track::open_symphonia;

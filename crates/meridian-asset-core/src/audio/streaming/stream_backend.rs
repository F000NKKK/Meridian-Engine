//! The decoder behind a stream.

use crate::AudioClip;
use symphonia::core::codecs::audio::AudioDecoder;
use symphonia::core::formats::FormatReader;

pub(crate) enum StreamBackend {
    /// Compressed formats: a live symphonia demux/decode pipeline.
    Symphonia {
        format: Box<dyn FormatReader>,
        decoder: Box<dyn AudioDecoder>,
        track_id: u32,
        /// Decoded samples beyond the last served block boundary.
        pending: Vec<i16>,
        finished: bool,
    },
    /// WAV is already PCM — "streaming" serves views of the parsed data
    /// (there is nothing to incrementally decode, but the caller gets
    /// one uniform block API for every format).
    Wav { audio: AudioClip, cursor: usize },
}

impl std::fmt::Debug for StreamBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Symphonia {
                track_id,
                pending,
                finished,
                ..
            } => f
                .debug_struct("Symphonia")
                .field("track_id", track_id)
                .field("pending_samples", &pending.len())
                .field("finished", finished)
                .finish_non_exhaustive(),
            Self::Wav { audio, cursor } => f
                .debug_struct("Wav")
                .field("samples", &audio.samples.len())
                .field("cursor", cursor)
                .finish(),
        }
    }
}

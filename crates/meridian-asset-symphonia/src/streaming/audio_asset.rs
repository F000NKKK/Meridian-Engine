//! Audio that is either fully decoded or a stream.

use meridian_asset_core::AudioClip;

use super::StreamingAudioDecoder;

/// What [`open_audio`](super::open_audio) produced: the whole track, or an incremental
/// decoder to pull it from block by block.
#[derive(Debug)]
pub enum AudioAsset {
    Decoded(AudioClip),
    Streaming(StreamingAudioDecoder),
}

impl AudioAsset {
    pub fn sample_rate(&self) -> u32 {
        match self {
            AudioAsset::Decoded(audio) => audio.sample_rate,
            AudioAsset::Streaming(stream) => stream.sample_rate(),
        }
    }

    pub fn channels(&self) -> u16 {
        match self {
            AudioAsset::Decoded(audio) => audio.channels,
            AudioAsset::Streaming(stream) => stream.channels(),
        }
    }
}

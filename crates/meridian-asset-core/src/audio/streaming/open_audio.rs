//! Opens audio bytes under a decode strategy.

use super::AudioAsset;
use super::DecodeMode;
use super::DecodeStrategy;
use super::StreamingAudioDecoder;
use crate::DecodeError;

/// The strategy-driven front door: sniff the format by signature, then
/// decode eagerly or hand back a [`StreamingAudioDecoder`](super::StreamingAudioDecoder) per
/// `strategy` (see [`DecodeMode`](super::DecodeMode)). In `Auto` mode a track whose length
/// the container doesn't declare streams — an unbounded decode can't be
/// safely eager.
pub fn open_audio(bytes: &[u8], strategy: &DecodeStrategy) -> Result<AudioAsset, DecodeError> {
    let stream = StreamingAudioDecoder::new(bytes, strategy.block_frames)?;
    let eager = match strategy.mode {
        DecodeMode::ForceEager => true,
        DecodeMode::ForceStreaming => false,
        DecodeMode::Auto => stream
            .estimated_decoded_bytes()
            .is_some_and(|bytes| bytes <= strategy.auto_threshold_bytes),
    };
    if eager {
        Ok(AudioAsset::Decoded(stream.decode_all()?))
    } else {
        Ok(AudioAsset::Streaming(stream))
    }
}

//! The decode mode plus its tuning.

use super::DecodeMode;

/// Tunable decode strategy for [`open_audio`](super::open_audio).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DecodeStrategy {
    /// Whether to decode eagerly, stream, or choose by estimated size.
    pub mode: DecodeMode,
    /// `Auto` streams once the estimated decoded size (PCM `i16`)
    /// exceeds this. Default 16 MiB ≈ 1.5 min of 48 kHz stereo.
    pub auto_threshold_bytes: u64,
    /// PCM frames per [`StreamingAudioDecoder::next_block`](super::StreamingAudioDecoder::next_block) — the
    /// streaming buffer size. Default 4096 (~85 ms at 48 kHz).
    pub block_frames: usize,
}

impl Default for DecodeStrategy {
    fn default() -> Self {
        Self {
            mode: DecodeMode::Auto,
            auto_threshold_bytes: 16 * 1024 * 1024,
            block_frames: 4096,
        }
    }
}

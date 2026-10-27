//! Decoded audio samples.

/// Decoded PCM audio data, not yet uploaded to an audio device.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AudioClip {
    pub sample_rate: u32,
    pub channels: u16,
    /// Interleaved 16-bit signed samples.
    pub samples: Vec<i16>,
}

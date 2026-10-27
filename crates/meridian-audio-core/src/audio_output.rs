//! The device stream that mixed samples are pushed to.

use crate::SpeakerLayout;

/// A real audio output for a [`SpeakerLayout`](crate::SpeakerLayout): the bridge from this
/// crate's mixed samples to `audio-driver`'s device stream (the one
/// dependency edge a `*-core` is allowed on its own `*-driver` — see
/// docs/dependency-rules.md rule 1). Owns an open
/// `audio-driver::AudioStream` with one stream channel per speaker, in
/// speaker order — exactly the interleaving [`Mixer::render_interleaved`](crate::Mixer::render_interleaved)
/// produces.
#[derive(Debug)]
pub struct AudioOutput {
    stream: meridian_audio_driver::AudioStream,
}

impl AudioOutput {
    /// Opens the default output device with one channel per speaker in
    /// `layout`. Async because device enumeration is genuine I/O
    /// (`audio-driver::AudioDevice::new`, ADR 009); everything after the
    /// device exists is synchronous.
    ///
    /// `buffer_frames` bounds the stream's ring buffer (`None` = the
    /// driver's default, several hardware buffers deep). The ring is the
    /// audio's *latency*: samples mixed against a listener pose sit in
    /// it until the hardware drains them, so an interactive scene that
    /// re-mixes against a moving listener should request a small ring
    /// (the driver still clamps to what the hardware needs to run
    /// underrun-free).
    pub async fn open(
        layout: &SpeakerLayout,
        sample_rate: u32,
        buffer_frames: Option<u32>,
    ) -> Result<Self, meridian_audio_driver::AudioDeviceError> {
        let device = meridian_audio_driver::AudioDevice::new().await?;
        let stream = device.open_stream(meridian_audio_driver::AudioStreamConfig {
            sample_rate,
            channels: layout.speakers.len() as u16,
            buffer_frames,
        })?;
        Ok(Self { stream })
    }

    /// Pushes interleaved samples (as produced by
    /// [`Mixer::render_interleaved`](crate::Mixer::render_interleaved)) for playback. Blocks when the
    /// stream's ring buffer is full — the intended backpressure: a loop
    /// that renders a block and pushes it runs at the hardware's real
    /// playback rate.
    pub fn push_interleaved(&self, samples: &[f32]) {
        self.stream.push_samples(samples);
    }

    /// `true` if `sample_count` more samples fit without blocking.
    pub fn can_push(&self, sample_count: usize) -> bool {
        self.stream.can_push(sample_count)
    }
}

//! The port through which mixed samples leave the audio domain.

/// Where interleaved samples go to be heard: the audio context's output port.
///
/// The samples are those of [`Mixer::render_interleaved`](meridian_audio_domain::Mixer::render_interleaved):
/// one value per speaker per frame, in speaker order. Opening a device is the
/// adapter's business (it is I/O and `async`), so a sink only receives samples
/// once it exists. Adapters implement this trait (`meridian-audio-cpal` for
/// the platform device); [`testing::RecordingSink`](crate::testing::RecordingSink)
/// is the in-memory fake, and [`testing::run_contract`](crate::testing::run_contract)
/// is the suite every implementation passes.
pub trait AudioSink {
    /// Pushes interleaved samples for playback.
    ///
    /// May block while the sink is full: that backpressure paces a loop that
    /// renders a block and pushes it to the playback rate.
    fn push_interleaved(&self, samples: &[f32]);

    /// Whether `sample_count` more samples fit without blocking.
    ///
    /// Monotone: if `n` samples fit, so do fewer. Zero always fits.
    fn can_push(&self, sample_count: usize) -> bool;
}

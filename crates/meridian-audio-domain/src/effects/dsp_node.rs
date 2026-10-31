//! A processing node of the DSP graph.

/// A single DSP effect: mutates a buffer of samples in place.
pub trait DspNode: Send {
    /// Processes one interleaved sample buffer in place.
    fn process(&mut self, samples: &mut [f32]);
}

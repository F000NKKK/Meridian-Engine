//! A processing node of the DSP graph.

/// A single DSP effect: mutates a buffer of samples in place.
pub trait DspNode: Send {
    fn process(&mut self, samples: &mut [f32]);
}

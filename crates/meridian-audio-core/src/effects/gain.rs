//! Constant gain — the simplest [`DspNode`](super::DspNode).

use super::DspNode;

/// Scales every sample by a fixed factor.
#[derive(Debug, Clone, Copy)]
pub struct Gain(pub f32);

impl DspNode for Gain {
    fn process(&mut self, samples: &mut [f32]) {
        for s in samples.iter_mut() {
            *s *= self.0;
        }
    }
}

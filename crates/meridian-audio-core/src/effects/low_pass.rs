//! One-pole low-pass filter.

use super::DspNode;

/// A one-pole IIR low-pass filter: `state += alpha * (input - state)`.
/// `alpha` close to `1.0` barely filters; close to `0.0` filters heavily.
/// Useful for the well-known "distant sounds lose high frequencies"
/// effect (air absorption) — this crate doesn't wire that up
/// automatically from [`AttenuationModel`](crate::AttenuationModel), but
/// the piece is here.
#[derive(Debug, Clone, Copy)]
pub struct LowPassFilter {
    pub alpha: f32,
    state: f32,
}

impl LowPassFilter {
    pub fn new(alpha: f32) -> Self {
        Self { alpha, state: 0.0 }
    }
}

impl DspNode for LowPassFilter {
    fn process(&mut self, samples: &mut [f32]) {
        for s in samples.iter_mut() {
            self.state += self.alpha * (*s - self.state);
            *s = self.state;
        }
    }
}

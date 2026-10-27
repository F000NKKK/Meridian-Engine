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

    /// A filter for a cutoff frequency at a sample rate — the flexible
    /// front door when "alpha" is not the unit you think in:
    /// `alpha = 1 - exp(-2π * cutoff / rate)`.
    pub fn from_cutoff(cutoff_hz: f32, sample_rate: u32) -> Self {
        Self::new(1.0 - (-std::f32::consts::TAU * cutoff_hz / sample_rate.max(1) as f32).exp())
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

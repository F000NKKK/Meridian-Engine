//! Slew-rate limiting de-clicker — the safety net that keeps *any*
//! step discontinuity from reaching the ears as a click.

use super::DspNode;

/// A per-channel slew-rate limiter: caps how far the signal may move
/// between consecutive samples of the same channel. Clicks — buffer
/// seams, loop points, effect-parameter glitches, anything that steps
/// the waveform — are (near-)full-scale jumps between two samples;
/// music at ordinary levels moves far less per sample even in bright
/// transients. Capping the per-sample delta therefore flattens clicks
/// into short inaudible slopes while passing program material untouched.
///
/// Channel-aware: on an interleaved buffer, limiting must compare each
/// sample against the *previous sample of the same channel* — comparing
/// against the neighboring interleaved sample would "limit" the stereo
/// image itself. Construct with the stream's channel count and process
/// interleaved blocks directly.
///
/// `max_step` is the cap per sample, in full-scale units. The default
/// `0.35` at 48 kHz flattens a full-scale step in ~3 samples (far below
/// audibility as a click) while sitting well above the per-sample slew
/// of loud musical content. Lower values de-click harder but start to
/// soften genuine treble transients; it's a knob, not a constant.
#[derive(Debug, Clone)]
pub struct Declicker {
    channels: usize,
    pub max_step: f32,
    last: Vec<f32>,
}

impl Declicker {
    pub fn new(channels: usize) -> Self {
        Self::with_max_step(channels, 0.35)
    }

    pub fn with_max_step(channels: usize, max_step: f32) -> Self {
        let channels = channels.max(1);
        Self {
            channels,
            max_step: max_step.max(1e-4),
            last: vec![0.0; channels],
        }
    }
}

impl DspNode for Declicker {
    fn process(&mut self, samples: &mut [f32]) {
        for (index, sample) in samples.iter_mut().enumerate() {
            let channel = index % self.channels;
            let delta = (*sample - self.last[channel]).clamp(-self.max_step, self.max_step);
            *sample = self.last[channel] + delta;
            self.last[channel] = *sample;
        }
    }
}

#[cfg(test)]
mod declicker_tests {
    use super::*;

    #[test]
    fn flattens_a_full_scale_step_into_a_slope() {
        let mut d = Declicker::new(1);
        let mut samples = vec![0.0f32; 4];
        samples.extend_from_slice(&[1.0; 8]); // hard step: the click shape
        d.process(&mut samples);
        for pair in samples.windows(2) {
            assert!(
                (pair[1] - pair[0]).abs() <= 0.35 + 1e-6,
                "step {} survived",
                (pair[1] - pair[0]).abs()
            );
        }
        // ...but the signal still reaches the target level.
        assert!((samples.last().unwrap() - 1.0).abs() < 1e-6);
    }

    #[test]
    fn passes_ordinary_program_material_untouched() {
        // A loud 1 kHz sine at 48 kHz moves at most ~0.1 per sample —
        // well under the cap, so the limiter must be bit-transparent.
        let mut d = Declicker::new(1);
        let original: Vec<f32> = (0..480)
            .map(|i| (i as f32 * std::f32::consts::TAU * 1000.0 / 48_000.0).sin() * 0.8)
            .collect();
        let mut processed = original.clone();
        d.process(&mut processed);
        for (a, b) in original.iter().zip(&processed) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn interleaved_channels_are_limited_independently() {
        // Hard-panned stereo: left constant, right stepping. A limiter
        // that ignored channels would smear them into each other.
        let mut d = Declicker::new(2);
        let mut samples = vec![0.5, 0.0, 0.5, 1.0, 0.5, 0.0, 0.5, 1.0];
        d.process(&mut samples);
        for frame in samples.chunks_exact(2).skip(1) {
            assert!((frame[0] - 0.5).abs() < 1e-6, "left channel disturbed");
        }
    }
}

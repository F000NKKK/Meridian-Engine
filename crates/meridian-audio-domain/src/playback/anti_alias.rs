//! The low-pass applied before decimating.

use std::f64::consts::TAU;

/// Fraction of the output Nyquist frequency kept by the filter.
const PASSBAND: f64 = 0.9;
/// Quality factors of the two sections of a 4th-order Butterworth low-pass.
const SECTION_Q: [f64; 2] = [0.541_196_1, 1.306_562_9];

/// One direct-form-I biquad section.
#[derive(Debug, Clone, Copy)]
struct Section {
    b0: f64,
    b1: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Section {
    /// A low-pass at `cutoff` (a fraction of the sample rate) with quality `q`.
    fn low_pass(cutoff: f64, q: f64) -> Self {
        let w0 = TAU * cutoff;
        let (sin, cos) = w0.sin_cos();
        let alpha = sin / (2.0 * q);
        let a0 = 1.0 + alpha;
        Self {
            b0: (1.0 - cos) / 2.0 / a0,
            b1: (1.0 - cos) / a0,
            a1: -2.0 * cos / a0,
            a2: (1.0 - alpha) / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn process(&mut self, x: f64) -> f64 {
        let y = self.b0 * (x + self.x2) + self.b1 * self.x1 - self.a1 * self.y1 - self.a2 * self.y2;
        (self.x2, self.x1) = (self.x1, x);
        (self.y2, self.y1) = (self.y1, y);
        y
    }
}

/// A 4th-order Butterworth low-pass cutting just below the output Nyquist
/// frequency, so that decimating by `ratio` folds almost nothing back.
#[derive(Debug, Clone, Copy)]
pub(super) struct AntiAlias {
    sections: [Section; 2],
}

impl AntiAlias {
    /// The filter for converting by `ratio` input samples per output sample
    /// (above 1).
    pub(super) fn for_ratio(ratio: f64) -> Self {
        let cutoff = 0.5 * PASSBAND / ratio;
        Self {
            sections: SECTION_Q.map(|q| Section::low_pass(cutoff, q)),
        }
    }

    pub(super) fn process(&mut self, sample: f32) -> f32 {
        let filtered = self
            .sections
            .iter_mut()
            .fold(f64::from(sample), |x, section| section.process(x));
        #[expect(
            clippy::cast_possible_truncation,
            reason = "audio samples are stored as f32"
        )]
        let out = filtered as f32;
        out
    }
}

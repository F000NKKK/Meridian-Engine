//! A source converted to another sample rate.

use super::{SampleSource, anti_alias::AntiAlias};

/// Samples pulled from the wrapped source per refill.
const CHUNK: usize = 256;

/// Plays `source` (recorded at `from_hz`) at `to_hz` by linear interpolation
/// between neighbouring input samples. Cheap and adequate for music and
/// effects. [`new`](Self::new) does not low-pass before decimating, so a
/// large downward conversion aliases; [`anti_aliased`](Self::anti_aliased)
/// filters first. Interpolation needs the following sample, so a finite
/// source ends one input sample early.
#[derive(Debug)]
pub struct ResampledSource<S> {
    source: S,
    /// Input samples consumed per output sample.
    ratio: f64,
    /// Position between `previous` (0.0) and `next` (1.0).
    phase: f64,
    previous: f32,
    next: f32,
    chunk: Vec<f32>,
    chunk_pos: usize,
    filter: Option<AntiAlias>,
    primed: bool,
    ended: bool,
}

impl<S: SampleSource> ResampledSource<S> {
    /// Wraps `source`, converting from `from_hz` to `to_hz`. A zero rate
    /// leaves the source unconverted.
    #[must_use]
    pub fn new(source: S, from_hz: u32, to_hz: u32) -> Self {
        let ratio = if from_hz == 0 || to_hz == 0 {
            1.0
        } else {
            f64::from(from_hz) / f64::from(to_hz)
        };
        Self {
            source,
            ratio,
            phase: 0.0,
            previous: 0.0,
            next: 0.0,
            chunk: Vec::new(),
            chunk_pos: 0,
            filter: None,
            primed: false,
            ended: false,
        }
    }

    /// Like [`new`](Self::new), but a downward conversion first runs the
    /// input through a low-pass just below the output Nyquist frequency, so
    /// content the new rate cannot represent is removed instead of folded
    /// back as alias tones. Upward and equal-rate conversions are unfiltered.
    #[must_use]
    pub fn anti_aliased(source: S, from_hz: u32, to_hz: u32) -> Self {
        let mut resampled = Self::new(source, from_hz, to_hz);
        if resampled.ratio > 1.0 {
            resampled.filter = Some(AntiAlias::for_ratio(resampled.ratio));
        }
        resampled
    }

    /// The next input sample, or `None` once the source has ended.
    fn pull(&mut self) -> Option<f32> {
        if self.chunk_pos == self.chunk.len() {
            self.chunk.resize(CHUNK, 0.0);
            let read = self.source.read(&mut self.chunk);
            self.chunk.truncate(read);
            self.chunk_pos = 0;
        }
        let sample = self.chunk.get(self.chunk_pos).copied()?;
        self.chunk_pos += 1;
        Some(match &mut self.filter {
            Some(filter) => filter.process(sample),
            None => sample,
        })
    }

    /// Reads the first two input samples; `false` if the source has fewer.
    fn prime(&mut self) -> bool {
        let (Some(first), Some(second)) = (self.pull(), self.pull()) else {
            return false;
        };
        (self.previous, self.next) = (first, second);
        self.primed = true;
        true
    }

    /// Moves the interpolation window forward while `phase` is past `next`.
    fn advance_window(&mut self) -> bool {
        while self.phase >= 1.0 {
            let Some(sample) = self.pull() else {
                return false;
            };
            (self.previous, self.next) = (self.next, sample);
            self.phase -= 1.0;
        }
        true
    }
}

impl<S: SampleSource> SampleSource for ResampledSource<S> {
    fn read(&mut self, out: &mut [f32]) -> usize {
        if self.ended || (!self.primed && !self.prime()) {
            self.ended = true;
            return 0;
        }
        for (written, slot) in out.iter_mut().enumerate() {
            if !self.advance_window() {
                self.ended = true;
                return written;
            }
            *slot = self.previous + (self.next - self.previous) * self.phase as f32;
            self.phase += self.ratio;
        }
        out.len()
    }
}

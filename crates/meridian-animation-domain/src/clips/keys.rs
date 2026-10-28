//! The keyframes of one track.

use super::interpolate::{hermite, lerp, normalised, slerp};
use super::{ClipError, Interpolation};

/// Times and values of one animated property, sampled between keys by its
/// [`Interpolation`]. A cubic spline holds three values per key, in-tangent,
/// value and out-tangent, in that order.
#[derive(Debug, Clone, PartialEq)]
pub struct Keys<const N: usize> {
    times: Vec<f32>,
    values: Vec<[f32; N]>,
    interpolation: Interpolation,
}

impl<const N: usize> Keys<N> {
    /// Keys at `times` (seconds, strictly increasing) holding `values`.
    ///
    /// # Errors
    ///
    /// [`ClipError`] for no keys, a value count that does not match the keys
    /// and `interpolation`, or times that are not finite and increasing.
    pub fn new(
        times: Vec<f32>,
        values: Vec<[f32; N]>,
        interpolation: Interpolation,
    ) -> Result<Self, ClipError> {
        if times.is_empty() {
            return Err(ClipError::NoKeys);
        }
        if values.len() != times.len() * interpolation.values_per_key() {
            return Err(ClipError::ValueCount {
                keys: times.len(),
                values: values.len(),
            });
        }
        let increasing = times.windows(2).all(|pair| pair[0] < pair[1]);
        if let Some(index) = times.iter().position(|time| !time.is_finite()) {
            return Err(ClipError::TimesNotIncreasing { index });
        }
        if !increasing {
            let index = times
                .windows(2)
                .position(|pair| pair[0] >= pair[1])
                .map_or(0, |at| at + 1);
            return Err(ClipError::TimesNotIncreasing { index });
        }
        Ok(Self {
            times,
            values,
            interpolation,
        })
    }

    /// The time of the last key.
    #[must_use]
    pub fn end_time(&self) -> f32 {
        self.times.last().copied().unwrap_or(0.0)
    }

    /// The value at key `key`, whatever the interpolation stores around it.
    fn value_at(&self, key: usize) -> [f32; N] {
        match self.interpolation {
            Interpolation::CubicSpline => self.values[3 * key + 1],
            Interpolation::Step | Interpolation::Linear => self.values[key],
        }
    }

    /// Where `time` falls: on a key's value, or `t` of the way from one key
    /// to the next. The first key's value holds before it, the last key's
    /// after it.
    fn locate(&self, time: f32) -> Location {
        let last = self.times.len() - 1;
        if time <= self.times[0] {
            return Location::Hold(0);
        }
        if time >= self.times[last] {
            return Location::Hold(last);
        }
        let after = self.times.partition_point(|&key| key <= time);
        let before = after - 1;
        let span = self.times[after] - self.times[before];
        Location::Between {
            before,
            after,
            t: (time - self.times[before]) / span,
            span,
        }
    }

    /// The value at `time`, blending a pair of keys with `blend` for the
    /// linear mode and `curve` (on the two keys' tangents and values) for
    /// the spline.
    fn sample_with(
        &self,
        time: f32,
        blend: fn([f32; N], [f32; N], f32) -> [f32; N],
        curve: fn([[f32; N]; 4], f32, f32) -> [f32; N],
    ) -> [f32; N] {
        let Location::Between {
            before,
            after,
            t,
            span,
        } = self.locate(time)
        else {
            return match self.locate(time) {
                Location::Hold(key) => self.value_at(key),
                Location::Between { before, .. } => self.value_at(before),
            };
        };
        match self.interpolation {
            Interpolation::Step => self.value_at(before),
            Interpolation::Linear => blend(self.value_at(before), self.value_at(after), t),
            Interpolation::CubicSpline => curve(
                [
                    self.values[3 * before + 1],
                    self.values[3 * before + 2],
                    self.values[3 * after + 1],
                    self.values[3 * after],
                ],
                t,
                span,
            ),
        }
    }
}

impl Keys<3> {
    /// The vector at `time`.
    pub(super) fn sample(&self, time: f32) -> [f32; 3] {
        self.sample_with(time, lerp, hermite)
    }
}

impl Keys<4> {
    /// The unit quaternion at `time`.
    pub(super) fn sample(&self, time: f32) -> [f32; 4] {
        self.sample_with(time, slerp, |points, t, span| {
            normalised(hermite(points, t, span))
        })
    }
}

/// Where a time falls among a track's keys.
enum Location {
    /// On one key's value.
    Hold(usize),
    /// Between two keys.
    Between {
        before: usize,
        after: usize,
        t: f32,
        span: f32,
    },
}

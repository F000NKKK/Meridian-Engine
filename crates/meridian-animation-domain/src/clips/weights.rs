//! The keyframes of several morph weights that move together.

use super::{ClipError, Interpolation, Keys};

/// Times and values of the weights of one morph set, one track per target,
/// all on the same keys. Weights blend like any other property.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightKeys {
    targets: Vec<Keys<1>>,
}

impl WeightKeys {
    /// Keys at `times` holding `values`: for each key (and, for a cubic
    /// spline, each of in-tangent, value and out-tangent in turn) one weight
    /// per target.
    ///
    /// # Errors
    ///
    /// [`ClipError`] for no targets or keys, values that do not fit the keys
    /// and `interpolation`, or times that are not finite and increasing.
    pub fn new(
        times: &[f32],
        values: &[f32],
        targets: usize,
        interpolation: Interpolation,
    ) -> Result<Self, ClipError> {
        let sets = times.len() * interpolation.values_per_key();
        if targets == 0 || values.len() != sets * targets {
            return Err(ClipError::ValueCount {
                keys: times.len(),
                values: values.len(),
            });
        }
        let targets = (0..targets)
            .map(|target| {
                let column = (0..sets).map(|set| [values[set * targets + target]]);
                Keys::new(times.to_vec(), column.collect(), interpolation)
            })
            .collect::<Result<_, _>>()?;
        Ok(Self { targets })
    }

    /// The time of the last key.
    #[must_use]
    pub fn end_time(&self) -> f32 {
        self.targets.first().map_or(0.0, Keys::end_time)
    }

    /// Sets the leading `weights` to their values at `time`; targets the
    /// keys do not have are left alone.
    pub fn sample(&self, time: f32, weights: &mut [f32]) {
        for (weight, keys) in weights.iter_mut().zip(&self.targets) {
            *weight = keys.sample(time);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn two_targets_blend_independently_between_keys() {
        let keys = WeightKeys::new(&[0.0, 2.0], &[0.0, 1.0, 1.0, 0.0], 2, Interpolation::Linear)
            .expect("valid keys");
        let mut weights = [9.0, 9.0];
        keys.sample(1.0, &mut weights);
        assert_eq!(weights, [0.5, 0.5]);
        keys.sample(5.0, &mut weights);
        assert_eq!(weights, [1.0, 0.0], "the last key holds");
        assert!((keys.end_time() - 2.0).abs() < f32::EPSILON);
    }

    #[test]
    fn step_holds_the_earlier_key_and_spline_reads_the_middle_value() {
        let step = WeightKeys::new(&[0.0, 1.0], &[0.25, 0.75], 1, Interpolation::Step)
            .expect("valid keys");
        let mut weights = [0.0];
        step.sample(0.9, &mut weights);
        assert_eq!(weights, [0.25]);
        // One key: tangent in, value, tangent out.
        let spline = WeightKeys::new(&[0.0], &[0.0, 0.5, 0.0], 1, Interpolation::CubicSpline)
            .expect("valid keys");
        spline.sample(0.0, &mut weights);
        assert_eq!(weights, [0.5]);
    }

    #[test]
    fn extra_weights_are_left_alone_and_bad_shapes_are_refused() {
        let keys = WeightKeys::new(&[0.0], &[0.5], 1, Interpolation::Linear).expect("valid keys");
        let mut weights = [0.0, 7.0];
        keys.sample(0.0, &mut weights);
        assert_eq!(weights, [0.5, 7.0]);
        assert!(WeightKeys::new(&[0.0], &[0.5, 0.5], 1, Interpolation::Linear).is_err());
        assert!(WeightKeys::new(&[0.0], &[], 0, Interpolation::Linear).is_err());
    }
}

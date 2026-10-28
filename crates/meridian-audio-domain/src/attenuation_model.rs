//! Gain falling with distance from the listener.

use crate::AttenuationCurve;

/// Distance attenuation. `gain == 1.0` at or inside `reference_distance`,
/// decreasing beyond it along `curve` and clamped at `max_distance`. The
/// default curve is OpenAL's "inverse clamped distance", which never
/// vanishes to exactly zero.
#[derive(Debug, Clone, Copy)]
pub struct AttenuationModel {
    /// Distance up to which the gain is `1.0`.
    pub reference_distance: f32,
    /// How quickly the gain falls beyond the reference distance.
    pub rolloff: f32,
    /// Distance beyond which the gain stops changing.
    pub max_distance: f32,
    /// The shape of the fall between the two distances.
    pub curve: AttenuationCurve,
}

impl Default for AttenuationModel {
    fn default() -> Self {
        Self {
            reference_distance: 1.0,
            rolloff: 1.0,
            max_distance: 1000.0,
            curve: AttenuationCurve::Inverse,
        }
    }
}

impl AttenuationModel {
    pub fn gain(&self, distance: f32) -> f32 {
        let reference = self.reference_distance;
        let d = distance.clamp(reference, self.max_distance.max(reference));
        match self.curve {
            AttenuationCurve::Inverse => reference / (reference + self.rolloff * (d - reference)),
            AttenuationCurve::Linear => {
                let span = (self.max_distance - reference).max(f32::EPSILON);
                (1.0 - self.rolloff * (d - reference) / span).max(0.0)
            }
            AttenuationCurve::Exponential => (d / reference).powf(-self.rolloff),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(curve: AttenuationCurve) -> AttenuationModel {
        AttenuationModel {
            reference_distance: 1.0,
            rolloff: 1.0,
            max_distance: 11.0,
            curve,
        }
    }

    #[test]
    fn every_curve_is_unity_inside_the_reference_distance() {
        for curve in [
            AttenuationCurve::Inverse,
            AttenuationCurve::Linear,
            AttenuationCurve::Exponential,
        ] {
            assert_eq!(model(curve).gain(0.25), 1.0, "{curve:?}");
            assert_eq!(model(curve).gain(1.0), 1.0, "{curve:?}");
        }
    }

    #[test]
    fn the_linear_curve_falls_straight_to_silence_at_the_maximum_distance() {
        let linear = model(AttenuationCurve::Linear);

        assert!((linear.gain(6.0) - 0.5).abs() < 1e-6);
        assert_eq!(linear.gain(11.0), 0.0);
        assert_eq!(linear.gain(500.0), 0.0, "stays silent beyond the maximum");
    }

    #[test]
    fn the_exponential_curve_follows_a_power_law() {
        let exponential = model(AttenuationCurve::Exponential);

        assert!((exponential.gain(2.0) - 0.5).abs() < 1e-6);
        assert!((exponential.gain(4.0) - 0.25).abs() < 1e-6);
    }

    #[test]
    fn the_default_is_the_inverse_curve() {
        assert_eq!(AttenuationModel::default().curve, AttenuationCurve::Inverse);
        assert!((model(AttenuationCurve::Inverse).gain(2.0) - 0.5).abs() < 1e-6);
    }

    #[test]
    fn a_maximum_inside_the_reference_distance_does_not_divide_by_zero() {
        let degenerate = AttenuationModel {
            reference_distance: 5.0,
            rolloff: 1.0,
            max_distance: 5.0,
            curve: AttenuationCurve::Linear,
        };

        assert!(degenerate.gain(20.0).is_finite());
    }
}

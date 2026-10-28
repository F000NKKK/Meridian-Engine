//! [`LightKindForm`]: `rendering-domain`'s `LightKind` as files hold it.

use meridian_rendering_domain::LightKind;
use substrate_data::Schema;

/// How a light spreads, stored as a tagged variant.
#[derive(Debug, Clone, Copy, PartialEq, Schema)]
pub enum LightKindForm {
    /// Parallel light along the entity's local X axis.
    Directional,
    /// Light in every direction.
    Point {
        /// Distance at which the light has faded to nothing.
        range: f32,
    },
    /// A cone along the entity's local X axis.
    Spot {
        /// Distance at which the light has faded to nothing.
        range: f32,
        /// Half-angle in radians inside which the light is at full strength.
        inner_angle: f32,
        /// Half-angle in radians at which the light has faded to nothing.
        outer_angle: f32,
    },
}

impl From<LightKind> for LightKindForm {
    fn from(kind: LightKind) -> Self {
        match kind {
            LightKind::Directional => Self::Directional,
            LightKind::Point { range } => Self::Point { range },
            LightKind::Spot {
                range,
                inner_angle,
                outer_angle,
            } => Self::Spot {
                range,
                inner_angle,
                outer_angle,
            },
        }
    }
}

impl From<LightKindForm> for LightKind {
    fn from(form: LightKindForm) -> Self {
        match form {
            LightKindForm::Directional => Self::Directional,
            LightKindForm::Point { range } => Self::Point { range },
            LightKindForm::Spot {
                range,
                inner_angle,
                outer_angle,
            } => Self::Spot {
                range,
                inner_angle,
                outer_angle,
            },
        }
    }
}

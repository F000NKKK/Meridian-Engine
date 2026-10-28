//! How a light spreads.

/// How a [`LightSource`](crate::LightSource) spreads its light.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LightKind {
    /// Parallel light along the entity's local X axis (the sun).
    Directional,
    /// Light in every direction from the entity's position.
    Point {
        /// Distance at which the light has faded to nothing.
        range: f32,
    },
    /// A cone along the entity's local X axis from its position.
    Spot {
        /// Distance at which the light has faded to nothing.
        range: f32,
        /// Half-angle in radians inside which the light is at full strength.
        inner_angle: f32,
        /// Half-angle in radians at which the light has faded to nothing.
        outer_angle: f32,
    },
}

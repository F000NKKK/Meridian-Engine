//! What fills the frame behind a scene's geometry.

/// The backdrop of a [`Scene3d`](crate::Scene3d): drawn first, so every
/// renderable covers it, and never lit or shadowed.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Background {
    /// The frame's clear colour.
    #[default]
    Clear,
    /// The environment the recorder lights with (ADR 039), seen through the
    /// camera. `blur` picks how soft: `0.0` is the sharpest prefiltered level,
    /// `1.0` the roughest. The clear colour shows while no environment is set.
    Environment {
        /// `0.0` sharp to `1.0` fully blurred.
        blur: f32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Scene3d;

    #[test]
    fn a_scene_shows_the_clear_colour_unless_told_otherwise() {
        assert_eq!(Scene3d::default().background, Background::Clear);
    }
}

//! Everything one view writes into the shared lit uniform.

use super::ShadowLight;
use crate::Camera;
use crate::scene::{Light, Scene3d};

/// The inputs of the lit shaders' shared uniform buffer for one view.
#[derive(Debug, Clone, Copy)]
pub struct ViewUniforms<'a> {
    /// The view's camera.
    pub camera: &'a Camera,
    /// The light that casts this view's shadows.
    pub shadow: ShadowLight,
    /// Ambient color below the horizon.
    pub ambient_ground: [f32; 3],
    /// Ambient color above the horizon.
    pub ambient_sky: [f32; 3],
    /// The view's lights; those beyond [`MAX_LIGHTS`](super::MAX_LIGHTS) are dropped.
    pub lights: &'a [Light],
}

impl<'a> ViewUniforms<'a> {
    /// The uniforms for drawing `scene` with `shadow`.
    pub fn of_scene(scene: &'a Scene3d, shadow: ShadowLight) -> Self {
        Self {
            camera: &scene.camera,
            shadow,
            ambient_ground: scene.ambient_ground,
            ambient_sky: scene.ambient_sky,
            lights: &scene.lights,
        }
    }
}

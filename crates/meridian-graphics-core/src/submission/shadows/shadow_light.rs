//! The shadow-casting light of one view.

use meridian_gac_core::Vec3;

use super::directional_shadow_camera;
use crate::scene::{Light, Scene3d};

/// The light that casts this view's shadows, as
/// `SceneRenderer::render_shadow_pass` (the adapter)
/// computed it: the light-space view-projection matrix, the light's travel
/// direction, and its index in the scene's lights. `caster` is `u32::MAX`
/// when the scene has no directional light, which the lit shaders' shadow
/// lookup never matches.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowLight {
    /// Light-space view-projection matrix.
    pub view_proj: [[f32; 4]; 4],
    /// The light's travel direction (unit vector).
    pub direction: Vec3,
    /// Index of the shadow-casting light in the scene's lights.
    pub caster: u32,
}

impl ShadowLight {
    /// The scene's first directional light as a shadow caster, or
    /// `caster = u32::MAX` (matched by no light) when there is none.
    #[must_use]
    pub fn for_scene(scene: &Scene3d) -> Self {
        let caster = scene
            .lights
            .iter()
            .position(|light| matches!(light, Light::Directional { .. }))
            .and_then(|index| u32::try_from(index).ok())
            .unwrap_or(u32::MAX);
        let direction = match scene.lights.get(caster as usize) {
            Some(Light::Directional { direction, .. }) => direction.transform_vector(Vec3::X),
            _ => Vec3::new(0.0, -1.0, 0.0),
        };
        let camera = directional_shadow_camera(direction, Vec3::ZERO);
        Self {
            view_proj: camera.view_projection_matrix(),
            direction,
            caster,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_scene_without_a_directional_light_has_no_caster() {
        let shadow = ShadowLight::for_scene(&Scene3d::default());
        assert_eq!(shadow.caster, u32::MAX);
        assert_eq!(shadow.direction, Vec3::new(0.0, -1.0, 0.0));
    }
}

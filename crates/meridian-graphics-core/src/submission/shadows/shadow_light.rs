//! The shadow-casting light of one view.

use meridian_gac_core::Vec3;

/// The light that casts this view's shadows, as
/// [`SceneRenderer::render_shadow_pass`](crate::submission::SceneRenderer::render_shadow_pass)
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

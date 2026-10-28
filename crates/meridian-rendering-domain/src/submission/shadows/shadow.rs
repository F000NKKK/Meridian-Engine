//! The camera that sees the fixed shadow volume.

use meridian_gac::{Motor3, Projection, Vec3};

use crate::{Camera, look_at_rotor};

/// Half-extent (world units) of the fixed orthographic volume a view gets
/// when its camera has no perspective frustum to fit cascades to (see
/// [`ShadowCascades::fixed`](super::ShadowCascades::fixed)): centred on the
/// world origin, generous enough for the example scenes (floor half-extent
/// 14) and not fitted to anything.
pub const SHADOW_VOLUME_HALF_EXTENT: f32 = 20.0;

/// Builds the [`Camera`] a directional light "sees" its shadow-casting
/// geometry through: an orthographic box of side
/// `2 * SHADOW_VOLUME_HALF_EXTENT` centered on `scene_center`, looking
/// along `light_direction`. Reused as an ordinary [`Camera`] (see that
/// type's own `view_projection_matrix`) so the shadow map's light-space
/// matrix goes through the exact same, independently-tested view/
/// projection math every other camera in this workspace does — not a
/// hand-rolled parallel matrix path.
pub fn directional_shadow_camera(light_direction: Vec3, scene_center: Vec3) -> Camera {
    let forward = light_direction.normalize();
    // The light "camera" sits behind the scene, looking toward it along
    // `forward` — far enough back that `scene_center` sits comfortably
    // inside the ortho box's near/far range for any `forward` direction.
    let eye = scene_center - forward * (SHADOW_VOLUME_HALF_EXTENT * 2.0);
    let rotor = look_at_rotor(eye, scene_center);
    let h = SHADOW_VOLUME_HALF_EXTENT;
    Camera {
        frame: Motor3::from_rotation_translation(rotor, eye),
        projection: Projection::orthographic((-h, h), (-h, h), (0.1, h * 4.0)),
    }
}

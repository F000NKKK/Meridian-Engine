//! A camera frame plus projection.

use meridian_gac_core::{Motor3, Projection, Rotor, Vec3, mat4_mul};

/// Remaps `gac-core`'s local camera axes (forward `+X`, up `+Y`, right
/// `+Z` — the same listener-local convention `audio-core` already commits
/// to, reused here so a character's camera and ears agree on "forward"
/// without either subsystem inventing its own axis convention) onto
/// `Projection`'s documented view space (forward `-Z`, up `+Y`, right
/// `+X`). `gac-core` itself has no "forward" concept (see
/// docs/gac-design.md); this remap is a graphics-specific convention
/// choice and stays in `graphics-core`, not `gac-core`.
const LOCAL_TO_VIEW_REMAP: [[f32; 4]; 4] = [
    [0.0, 0.0, -1.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// The `Rotor` whose local-forward `+X` axis (`gac-core`'s convention,
/// see `LOCAL_TO_VIEW_REMAP`'s doc comment) points from `eye` toward
/// `target` — the shared "aim this frame at that point" building block
/// [`Camera::frame`]/a directional light's frame both need. Falls back
/// to a fixed rotation when `eye == target` or `target` is exactly
/// behind `eye`'s forward axis (an undefined/degenerate cross product
/// otherwise) rather than producing a `NaN` rotor.
pub fn look_at_rotor(eye: Vec3, target: Vec3) -> Rotor {
    let forward = (target - eye).normalize();
    let local_forward = Vec3::X;
    let cos_angle = local_forward.dot(forward).clamp(-1.0, 1.0);
    if cos_angle > 0.9999 {
        return Rotor::identity();
    }
    if cos_angle < -0.9999 {
        return Rotor::from_axis_angle(Vec3::Y, core::f32::consts::PI);
    }
    let axis = local_forward.cross(forward).normalize();
    Rotor::from_axis_angle(axis, cos_angle.acos())
}

/// A camera's view + projection: `frame` is the camera's world transform
/// (local-forward `+X`, see `LOCAL_TO_VIEW_REMAP`'s doc comment below),
/// `projection` is the view-to-clip mapping built via
/// [`Projection::perspective`] or [`Projection::orthographic`].
#[derive(Debug, Clone, Copy, Default)]
pub struct Camera {
    pub frame: Motor3,
    pub projection: Projection,
}

impl Camera {
    /// World-space to view-space matrix: the camera's world frame
    /// inverted (world-to-camera-local), then remapped from `gac-core`'s
    /// local forward-`+X` convention to `Projection`'s forward-`-Z` view
    /// space.
    pub fn view_matrix(&self) -> [[f32; 4]; 4] {
        mat4_mul(LOCAL_TO_VIEW_REMAP, self.frame.inverse().to_mat4())
    }

    /// World-space to clip-space matrix: `projection * view`.
    pub fn view_projection_matrix(&self) -> [[f32; 4]; 4] {
        mat4_mul(self.projection.0, self.view_matrix())
    }
}

//! Directional shadow depth shader and the camera that fits the shadow volume.

use meridian_gac_core::{Motor3, Projection, Vec3};

use crate::{Camera, look_at_rotor};

/// The shadow map's own depth-only pass: transforms each vertex by the
/// shadow-casting light's own view-projection matrix (not the camera's)
/// and writes nothing but depth — no fragment stage at all (see
/// `graphics-driver::Device::create_shadow_pipeline`, which builds this
/// shader with `fragment: None`). Reused unmodified for both colored and
/// textured draws via `shadow_from_colored_layout`/
/// `shadow_from_textured_layout` — this shader only ever reads
/// `@location(0) position`, so the rest of either family's vertex bytes
/// (already baked into the same buffer the main pass uses — see
/// [`SceneRenderer::render_shadow_pass`](crate::submission::SceneRenderer::render_shadow_pass)) are simply never touched.
pub const SHADOW_DEPTH_SHADER_WGSL: &str = r#"
struct ShadowUniforms {
    light_view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> u: ShadowUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> @builtin(position) vec4<f32> {
    return u.light_view_proj * vec4<f32>(in.position, 1.0);
}
"#;

/// Fixed half-extent (world units) of the directional shadow map's
/// orthographic volume, centered on the world origin — a disclosed
/// simplification: a real engine fits this box to the camera frustum
/// each frame (a "shadow cascade"); this workspace's scenes are small
/// and roughly origin-centered, so one fixed, generous box covers both
/// `physic_figures` (floor half-extent 14) and `magic_figures` (orbit
/// radius 3.2, floor half-extent 14) without per-frame recomputation.
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
        projection: Projection::orthographic(-h, h, -h, h, 0.1, h * 4.0),
    }
}

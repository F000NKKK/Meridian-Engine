//! The shadow map's depth-only WGSL.

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

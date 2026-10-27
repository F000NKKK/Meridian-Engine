//! The shadow map's depth-only WGSL.

/// The shadow map's own depth-only pass: transforms each vertex by the
/// shadow-casting light's own view-projection matrix (not the camera's)
/// and writes nothing but depth — no fragment stage at all (see
/// `rendering-wgpu::Device::create_shadow_pipeline`, which builds this
/// shader with `fragment: None`). Reused unmodified for every batch: it reads `@location(0) position` from the mesh buffer and
/// the model matrix (`@location(3..=6)`) from the instance buffer, and
/// nothing else of either (see
/// [`SceneRenderer::render_shadow_pass`](crate::submission::SceneRenderer::render_shadow_pass)).
pub const SHADOW_DEPTH_SHADER_WGSL: &str = r#"
struct ShadowUniforms {
    light_view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> u: ShadowUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(3) model_0: vec4<f32>,
    @location(4) model_1: vec4<f32>,
    @location(5) model_2: vec4<f32>,
    @location(6) model_3: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> @builtin(position) vec4<f32> {
    let model = mat4x4<f32>(in.model_0, in.model_1, in.model_2, in.model_3);
    return u.light_view_proj * (model * vec4<f32>(in.position, 1.0));
}
"#;

//! The unlit WGSL shader.

/// A flat, unlit pipeline: `view_proj * vec4(position, 1)`, output color
/// is the per-vertex color + emissive baked in at
/// [`SceneRenderer::prepare`](crate::submission::SceneRenderer::prepare) time (see the module doc for why it's
/// per-vertex, not a material uniform). Used for `Material::unlit`
/// materials with no resolvable albedo texture.
pub const UNLIT_SHADER_WGSL: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) emissive: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) emissive: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    out.emissive = in.emissive;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color.rgb + in.emissive, in.color.a);
}
"#;

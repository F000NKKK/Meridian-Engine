//! The textured WGSL shader.

/// An unlit, textured pipeline: samples `albedo_tex` and multiplies by
/// the per-vertex tint (the material's `base_color_factor`), plus
/// per-vertex emissive — baked in at [`SceneRenderer::prepare`](crate::submission::SceneRenderer::prepare) time
/// (see the module doc for why tint/emissive travel per-vertex rather
/// than through a second uniform). Used for `Material::unlit` materials
/// whose albedo resolves in a [`TextureRegistry`](crate::submission::TextureRegistry).
pub const TEXTURED_SHADER_WGSL: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;
@group(0) @binding(1)
var albedo_tex: texture_2d<f32>;
@group(0) @binding(2)
var albedo_sampler: sampler;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) emissive: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) tint: vec4<f32>,
    @location(2) emissive: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u.view_proj * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    out.tint = in.tint;
    out.emissive = in.emissive;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(albedo_tex, albedo_sampler, in.uv);
    return vec4<f32>(sampled.rgb * in.tint.rgb + in.emissive, sampled.a * in.tint.a);
}
"#;

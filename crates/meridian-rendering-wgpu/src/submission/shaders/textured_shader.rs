//! The textured WGSL shader.

/// An unlit, textured pipeline: samples `albedo_tex` and multiplies by
/// the per-instance tint (the material's `base_color_factor`), plus
/// per-instance emissive, both from the instance record
/// `bake_batches`
/// writes. Used for `Material::unlit` materials
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
    @location(3) model_0: vec4<f32>,
    @location(4) model_1: vec4<f32>,
    @location(5) model_2: vec4<f32>,
    @location(6) model_3: vec4<f32>,
    @location(7) color: vec4<f32>,
    @location(8) emissive: vec3<f32>,
    @location(9) gloss: vec2<f32>,
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
    let model = mat4x4<f32>(in.model_0, in.model_1, in.model_2, in.model_3);
    out.clip_position = u.view_proj * (model * vec4<f32>(in.position, 1.0));
    out.uv = in.uv;
    out.tint = in.color;
    out.emissive = in.emissive;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(albedo_tex, albedo_sampler, in.uv);
    return vec4<f32>(sampled.rgb * in.tint.rgb + in.emissive, sampled.a * in.tint.a);
}
"#;

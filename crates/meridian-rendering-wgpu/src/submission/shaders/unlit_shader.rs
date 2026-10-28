//! The unlit WGSL shader.

/// A flat, unlit pipeline: `view_proj * model * vec4(position, 1)`, output
/// color is the per-instance color + emissive from the instance record
/// `bake_batches`
/// writes. Used for `Material::unlit`
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
    @location(2) uv: vec2<f32>,
    @location(3) model_0: vec4<f32>,
    @location(4) model_1: vec4<f32>,
    @location(5) model_2: vec4<f32>,
    @location(6) model_3: vec4<f32>,
    @location(7) color: vec4<f32>,
    @location(8) emissive: vec3<f32>,
    @location(9) surface: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) emissive: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let model = mat4x4<f32>(in.model_0, in.model_1, in.model_2, in.model_3);
    out.clip_position = u.view_proj * (model * vec4<f32>(in.position, 1.0));
    out.color = in.color;
    out.emissive = in.emissive;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color.rgb + in.emissive, in.color.a);
}
"#;

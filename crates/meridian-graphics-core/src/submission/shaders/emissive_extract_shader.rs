//! The WGSL shader that extracts emissive light for bloom.

/// Emissive-only extraction: ignores lighting/base color entirely and
/// outputs just the baked-in emissive term — [`crate::bloom`]'s
/// bright-pass input. Shares the *same* vertex buffer bytes as whichever
/// main pipeline drew this instance (see `emissive_from_colored_layout`/
/// `emissive_from_textured_layout`): a material with `emissive =
/// [0,0,0]` (the default) contributes pure black here, which an additive
/// bloom composite treats as a no-op — no per-material filtering needed.
pub const EMISSIVE_EXTRACT_SHADER_WGSL: &str = r#"
struct Uniforms {
    view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) emissive: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) emissive: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u.view_proj * vec4<f32>(in.position, 1.0);
    out.emissive = in.emissive;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.emissive, 1.0);
}
"#;

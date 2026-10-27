//! The sprite WGSL shader source.

/// Transforms camera units to clip space with the camera's extents and
/// outputs premultiplied color: the sampled (straight) texel times the
/// vertex tint, with `rgb` scaled by alpha. `extents` is
/// `[left, right, bottom, top]`, so a `y`-down camera (`top < bottom`)
/// needs no special case.
pub const SPRITE_SHADER_WGSL: &str = r"
struct Camera {
    extents: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> camera: Camera;
@group(0) @binding(1)
var sprite_tex: texture_2d<f32>;
@group(0) @binding(2)
var sprite_sampler: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    let e = camera.extents;
    let x = 2.0 * (in.position.x - e.x) / (e.y - e.x) - 1.0;
    let y = 2.0 * (in.position.y - e.z) / (e.w - e.z) - 1.0;
    var out: VertexOutput;
    out.clip_position = vec4<f32>(x, y, 0.0, 1.0);
    out.uv = in.uv;
    out.color = in.color;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let texel = textureSample(sprite_tex, sprite_sampler, in.uv) * in.color;
    return vec4<f32>(texel.rgb * texel.a, texel.a);
}
";

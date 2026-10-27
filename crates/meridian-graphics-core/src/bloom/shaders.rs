//! WGSL for the bloom blur and composite passes.

/// A full-screen-triangle vertex shader shared by both blur directions
/// and the composite pass — see `graphics-driver`'s
/// `Device::create_fullscreen_pipeline` doc comment for why no vertex
/// buffer is needed.
const FULLSCREEN_VERTEX_WGSL: &str = r#"
struct VsOut {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(@builtin(vertex_index) index: u32) -> VsOut {
    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0, -1.0),
        vec2<f32>(3.0, -1.0),
        vec2<f32>(-1.0, 3.0),
    );
    let p = positions[index];
    var out: VsOut;
    out.clip_position = vec4<f32>(p, 0.0, 1.0);
    out.uv = vec2<f32>((p.x + 1.0) * 0.5, 1.0 - (p.y + 1.0) * 0.5);
    return out;
}
"#;

/// A 5-tap separable Gaussian blur (the classic `LearnOpenGL` weights),
/// applied along `u.direction` — call once with a horizontal direction,
/// once with vertical, ping-ponging the source/destination textures, for
/// a full 2D blur at roughly half the cost of a naive 2D kernel.
pub(crate) fn blur_shader_wgsl() -> String {
    format!(
        r#"
{FULLSCREEN_VERTEX_WGSL}

struct BlurUniforms {{
    texel_size: vec2<f32>,
    direction: vec2<f32>,
}};

@group(0) @binding(0)
var<uniform> u: BlurUniforms;
@group(0) @binding(1)
var src_tex: texture_2d<f32>;
@group(0) @binding(2)
var src_sampler: sampler;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {{
    let weights = array<f32, 5>(0.227027, 0.1945946, 0.1216216, 0.054054, 0.016216);
    // A 5-tap kernel at a 1-texel stride only spreads a few pixels wide
    // at full render resolution — imperceptible next to a whole shape.
    // `spread` widens each tap's stride, but too large a stride relative
    // to only 5 taps undersamples the falloff into visible blocky rings
    // ("voxel" banding) instead of a smooth gradient. `BloomPass::apply`
    // runs this same shader through two full horizontal+vertical
    // iterations instead — composing two moderate blurs (each Gaussian
    // in its own right) still produces a smooth, wide result, the same
    // way stacking box blurs approximates a larger Gaussian, without
    // the gaps a single very-wide-stride pass leaves between samples.
    let spread = 2.2;
    var result = textureSample(src_tex, src_sampler, in.uv).rgb * weights[0];
    for (var i = 1; i < 5; i = i + 1) {{
        let offset = u.direction * u.texel_size * f32(i) * spread;
        result += textureSample(src_tex, src_sampler, in.uv + offset).rgb * weights[i];
        result += textureSample(src_tex, src_sampler, in.uv - offset).rgb * weights[i];
    }}
    return vec4<f32>(result, 1.0);
}}
"#
    )
}

/// Samples the blurred bright texture, scaled by `u.intensity`, additive
/// onto whatever [`meridian_graphics_driver::CommandBuffer::begin_render_pass_loaded`]
/// targets.
pub(crate) fn composite_shader_wgsl() -> String {
    format!(
        r#"
{FULLSCREEN_VERTEX_WGSL}

struct CompositeUniforms {{
    intensity: vec4<f32>,
}};

@group(0) @binding(0)
var<uniform> u: CompositeUniforms;
@group(0) @binding(1)
var bloom_tex: texture_2d<f32>;
@group(0) @binding(2)
var bloom_sampler: sampler;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {{
    let sampled = textureSample(bloom_tex, bloom_sampler, in.uv);
    return vec4<f32>(sampled.rgb * u.intensity.x, 1.0);
}}
"#
    )
}

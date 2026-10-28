//! WGSL for the tone mapping pass.

use crate::FULLSCREEN_VERTEX_WGSL;

/// Reads the HDR scene, scales it by the exposure and compresses it with the
/// curve `u.params.y` names (`TonemapOperator::index`). The swapchain view is
/// sRGB, so the hardware applies the display transfer on write.
pub(crate) fn tonemap_shader_wgsl() -> String {
    format!(
        r"
{FULLSCREEN_VERTEX_WGSL}

struct TonemapUniforms {{
    params: vec4<f32>,
}};

@group(0) @binding(0)
var<uniform> u: TonemapUniforms;
@group(0) @binding(1)
var scene_tex: texture_2d<f32>;
@group(0) @binding(2)
var scene_sampler: sampler;

fn aces(x: vec3<f32>) -> vec3<f32> {{
    return clamp((x * (2.51 * x + 0.03)) / (x * (2.43 * x + 0.59) + 0.14),
        vec3<f32>(0.0), vec3<f32>(1.0));
}}

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {{
    let sampled = textureSample(scene_tex, scene_sampler, in.uv);
    let exposed = max(sampled.rgb * u.params.x, vec3<f32>(0.0));
    var mapped = clamp(exposed, vec3<f32>(0.0), vec3<f32>(1.0));
    if (u.params.y > 1.5) {{
        mapped = aces(exposed);
    }} else if (u.params.y > 0.5) {{
        mapped = exposed / (exposed + vec3<f32>(1.0));
    }}
    return vec4<f32>(mapped, 1.0);
}}
"
    )
}

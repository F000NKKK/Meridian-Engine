//! WGSL for the sky backdrop.

use crate::FULLSCREEN_VERTEX_WGSL;

/// Looks up the environment texture along each pixel's view ray. `right`, `up`
/// and `forward` are the camera's world axes, `params.xy` the half-extents of
/// the view plane at distance 1 and `params.z` the mip level to read. The
/// equirectangular mapping is the one of `environment_shader`'s
/// `environment_uv`.
pub(crate) fn sky_shader_wgsl() -> String {
    format!(
        r"
{FULLSCREEN_VERTEX_WGSL}

struct SkyUniforms {{
    right: vec4<f32>,
    up: vec4<f32>,
    forward: vec4<f32>,
    params: vec4<f32>,
}};

@group(0) @binding(0)
var<uniform> u: SkyUniforms;
@group(0) @binding(1)
var env_map: texture_2d<f32>;
@group(0) @binding(2)
var env_sampler: sampler;

@fragment
fn fs_main(in: VsOut) -> @location(0) vec4<f32> {{
    let ndc = vec2<f32>(in.uv.x * 2.0 - 1.0, 1.0 - in.uv.y * 2.0);
    let d = normalize(u.forward.xyz
        + u.right.xyz * (ndc.x * u.params.x)
        + u.up.xyz * (ndc.y * u.params.y));
    let uv = vec2<f32>(
        0.5 + atan2(d.x, -d.z) * 0.15915494,
        0.5 - asin(clamp(d.y, -1.0, 1.0)) * 0.31830989,
    );
    return vec4<f32>(textureSampleLevel(env_map, env_sampler, uv, u.params.z).rgb, 1.0);
}}
"
    )
}

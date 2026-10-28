//! WGSL for image-based lighting, spliced into the lit shaders.

/// The environment lookups `shade` uses (ADR 035). They read the `u.env_sh`
/// and `u.env_params` uniform fields and the `env_map`/`env_sampler`
/// bindings, which each lit shader flavor declares at its own binding numbers.
///
/// The equirectangular mapping is `meridian_rendering_domain::direction_to_uv`:
/// `u = 0.5 + atan2(x, -z) / 2pi`, `v = 0.5 - asin(y) / pi`. The nine
/// coefficients are irradiance / pi, so diffuse needs no further 1/pi.
pub(super) const ENVIRONMENT_WGSL: &str = r"
const INV_TWO_PI: f32 = 0.15915494;
const INV_PI: f32 = 0.31830989;

fn environment_uv(d: vec3<f32>) -> vec2<f32> {
    return vec2<f32>(
        0.5 + atan2(d.x, -d.z) * INV_TWO_PI,
        0.5 - asin(clamp(d.y, -1.0, 1.0)) * INV_PI,
    );
}

/// Cosine-weighted mean radiance of the hemisphere around `n`: the nine
/// spherical-harmonic terms of `EnvironmentMap::irradiance`.
fn environment_irradiance(n: vec3<f32>) -> vec3<f32> {
    return u.env_sh[0].rgb * 0.282095
        + u.env_sh[1].rgb * (0.488603 * n.y)
        + u.env_sh[2].rgb * (0.488603 * n.z)
        + u.env_sh[3].rgb * (0.488603 * n.x)
        + u.env_sh[4].rgb * (1.092548 * n.x * n.y)
        + u.env_sh[5].rgb * (1.092548 * n.y * n.z)
        + u.env_sh[6].rgb * (0.315392 * (3.0 * n.z * n.z - 1.0))
        + u.env_sh[7].rgb * (1.092548 * n.x * n.z)
        + u.env_sh[8].rgb * (0.546274 * (n.x * n.x - n.y * n.y));
}

/// The prefiltered radiance along `r` for `roughness`: mip level `i` was
/// filtered for roughness `i / max_lod`.
fn environment_reflection(r: vec3<f32>, roughness: f32) -> vec3<f32> {
    let lod = clamp(roughness, 0.0, 1.0) * u.env_params.y;
    return textureSampleLevel(env_map, env_sampler, environment_uv(r), lod).rgb;
}

/// Karis' analytic fit of the split-sum BRDF integral: the specular
/// colour a surface of `f0` and `roughness` reflects of a uniform white sky.
fn environment_brdf(f0: vec3<f32>, roughness: f32, n_dot_v: f32) -> vec3<f32> {
    let c0 = vec4<f32>(-1.0, -0.0275, -0.572, 0.022);
    let c1 = vec4<f32>(1.0, 0.0425, 1.04, -0.04);
    let r = roughness * c0 + c1;
    let a004 = min(r.x * r.x, exp2(-9.28 * n_dot_v)) * r.x + r.y;
    let ab = vec2<f32>(-1.04, 1.04) * a004 + r.zw;
    return f0 * ab.x + vec3<f32>(ab.y);
}
";

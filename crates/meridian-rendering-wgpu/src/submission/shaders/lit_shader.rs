//! The lit WGSL shaders and the light limit they are compiled for.

use super::environment_shader::ENVIRONMENT_WGSL;
use super::shadow_lookup_shader::SHADOW_LOOKUP_WGSL;

/// The uniform array's fixed capacity — see the module doc.
pub const MAX_LIGHTS: usize = 4;

/// Shared by every lit shader: the `Uniforms` struct layout (view-proj,
/// the shadow cascades' light-space view-projections, camera position, hemisphere
/// ambient, light count/array, which light index casts the shadow) plus
/// the metallic-roughness `shade` helper. The shadow lookup is its own text,
/// `shadow_lookup_shader`. WGSL has no `#include`, so this text is
/// spliced verbatim into each lit shader constant below — keeping one
/// source of truth here rather than hand-duplicating it at each call
/// site. The shadow *map*/*sampler* bindings themselves are declared per
/// shader flavor below, not here — colored and textured shaders already
/// use different binding numbers for their own resources (the textured
/// shader's albedo texture/sampler occupy bindings 1/2), so the shadow
/// map/sampler can't share one fixed binding number across both; each
/// flavor's `compute_shadow_factor` (in `shadow_lookup_shader`) reads those
/// declarations and hands `shade` a plain `f32`, keeping this shared text
/// binding-number-agnostic.
const LIT_UNIFORMS_AND_SHADING_WGSL: &str = r#"
struct LightData {
    // xyz: direction (kind 0) or world position (kind 1); w: kind (0 = directional, 1 = point).
    dir_or_pos: vec4<f32>,
    // rgb: color; a: intensity.
    color_intensity: vec4<f32>,
    // x: range (point lights only).
    range_pad: vec4<f32>,
};

struct Uniforms {
    view_proj: mat4x4<f32>,
    // The shadow cascades (ADR 037): one light-space matrix each, the view
    // distance at which each ends, and the normalised depth one shadow texel
    // of each spans.
    light_view_proj: array<mat4x4<f32>, 4>,
    cascade_far: vec4<f32>,
    cascade_texel_depth: vec4<f32>,
    camera_pos: vec4<f32>,
    ambient_ground: vec4<f32>,
    ambient_sky: vec4<f32>,
    // xyz: the shadow-casting light's own travel direction (surface ->
    // light is `-light_direction.xyz`) — kept as a plain vector rather
    // than indexing `lights[shadow_caster]` again, so the shadow lookup
    // never needs a dynamic array index.
    light_direction: vec4<f32>,
    light_count: vec4<u32>,
    // x: which `lights` index casts the shadow map (see `light_view_proj`);
    // `0xffffffff` means "no shadow-casting light in this scene".
    shadow_caster: vec4<u32>,
    lights: array<LightData, 4>,
    // The environment (ADR 035): nine irradiance/pi coefficients, then
    // x: 1 when an environment is set, y: its highest mip level.
    env_sh: array<vec4<f32>, 9>,
    env_params: vec4<f32>,
};

@group(0) @binding(0)
var<uniform> u: Uniforms;

/// A gradient between `ambient_ground` (facing straight down) and
/// `ambient_sky` (facing straight up) — see [`crate::scene::Scene3d`]'s
/// own doc comment for why this is a disclosed simplification of real
/// image-based lighting, not a convolved environment map.
fn hemisphere_ambient(world_normal: vec3<f32>) -> vec3<f32> {
    let t = world_normal.y * 0.5 + 0.5;
    return mix(u.ambient_ground.rgb, u.ambient_sky.rgb, t);
}

const PI: f32 = 3.14159265;

/// GGX (Trowbridge-Reitz) normal distribution for `alpha = roughness^2`.
fn distribution_ggx(n_dot_h: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let d = n_dot_h * n_dot_h * (a2 - 1.0) + 1.0;
    return a2 / (PI * d * d);
}

/// Height-correlated Smith visibility, which folds the geometry term and the
/// `4 * n_dot_l * n_dot_v` denominator into one factor.
fn visibility_smith_ggx(n_dot_l: f32, n_dot_v: f32, alpha: f32) -> f32 {
    let a2 = alpha * alpha;
    let from_view = n_dot_l * sqrt(n_dot_v * n_dot_v * (1.0 - a2) + a2);
    let from_light = n_dot_v * sqrt(n_dot_l * n_dot_l * (1.0 - a2) + a2);
    return 0.5 / max(from_view + from_light, 0.0001);
}

fn fresnel_schlick(v_dot_h: f32, f0: vec3<f32>) -> vec3<f32> {
    return f0 + (vec3<f32>(1.0) - f0) * pow(1.0 - v_dot_h, 5.0);
}

/// Metallic-roughness shading (ADR 032): `surface` is (roughness, metallic).
/// Light intensity keeps its pre-PBR meaning (a white diffuse surface facing
/// a light of intensity 1 is fully lit), so diffuse has no 1/pi and the
/// specular lobe is scaled by pi to match.
fn shade(world_pos: vec3<f32>, world_normal: vec3<f32>, base_color: vec3<f32>, surface: vec2<f32>, occlusion: f32, shadow_factor: f32) -> vec3<f32> {
    let n = normalize(world_normal);
    let view_dir = normalize(u.camera_pos.xyz - world_pos);
    let n_dot_v = max(dot(n, view_dir), 0.0001);
    let roughness = clamp(surface.x, 0.0, 1.0);
    let metallic = clamp(surface.y, 0.0, 1.0);
    // A mirror-smooth lobe is a single point; keep it finite.
    let alpha = max(roughness * roughness, 0.002);
    let f0 = mix(vec3<f32>(0.04), base_color, metallic);
    let diffuse_color = base_color * (1.0 - metallic);

    // With an environment (ADR 035) its irradiance lights the diffuse part
    // and its prefiltered reflection the specular part; without one the
    // sky/ground gradient stands in for it, looked up the same two ways,
    // with Fresnel damped by roughness.
    let reflected = reflect(-view_dir, n);
    var ambient: vec3<f32>;
    if (u.env_params.x > 0.5) {
        ambient = environment_irradiance(n) * diffuse_color
            + environment_reflection(reflected, roughness) * environment_brdf(f0, roughness, n_dot_v);
    } else {
        let ambient_fresnel = f0 + (max(vec3<f32>(1.0 - roughness), f0) - f0) * pow(1.0 - n_dot_v, 5.0);
        ambient = hemisphere_ambient(n) * diffuse_color + hemisphere_ambient(reflected) * ambient_fresnel;
    }
    var result = ambient * occlusion;

    let count = min(u.light_count.x, 4u);
    for (var i: u32 = 0u; i < count; i = i + 1u) {
        let light = u.lights[i];
        var light_dir: vec3<f32>;
        var attenuation: f32 = 1.0;
        if (light.dir_or_pos.w < 0.5) {
            light_dir = normalize(-light.dir_or_pos.xyz);
        } else {
            let to_light = light.dir_or_pos.xyz - world_pos;
            let dist = length(to_light);
            let range = max(light.range_pad.x, 0.0001);
            light_dir = to_light / max(dist, 0.0001);
            attenuation = clamp(1.0 - dist / range, 0.0, 1.0);
            attenuation = attenuation * attenuation;
        }
        if (i == u.shadow_caster.x) {
            attenuation = attenuation * shadow_factor;
        }
        let n_dot_l = max(dot(n, light_dir), 0.0);
        let half_dir = normalize(light_dir + view_dir);
        let n_dot_h = max(dot(n, half_dir), 0.0);
        let v_dot_h = max(dot(view_dir, half_dir), 0.0);
        let specular = distribution_ggx(n_dot_h, alpha)
            * visibility_smith_ggx(n_dot_l, n_dot_v, alpha)
            * fresnel_schlick(v_dot_h, f0)
            * PI;
        let radiance = light.color_intensity.rgb * light.color_intensity.a * attenuation;
        result = result + (diffuse_color + specular) * n_dot_l * radiance;
    }
    return result;
}
"#;

/// The lit fragment/vertex stage text spliced after the shared
/// uniforms and shading helpers.
const LIT_COLORED_BODY_WGSL: &str = r"

@group(0) @binding(1)
var shadow_map: texture_depth_2d;
@group(0) @binding(2)
var shadow_sampler: sampler_comparison;
@group(0) @binding(3)
var env_map: texture_2d<f32>;
@group(0) @binding(4)
var env_sampler: sampler;

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
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) emissive: vec3<f32>,
    @location(4) surface: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let model = mat4x4<f32>(in.model_0, in.model_1, in.model_2, in.model_3);
    let world = model * vec4<f32>(in.position, 1.0);
    let world_normal = (model * vec4<f32>(in.normal, 0.0)).xyz;
    out.clip_position = u.view_proj * world;
    out.world_position = world.xyz;
    out.world_normal = world_normal;
    out.color = in.color;
    out.emissive = in.emissive;
    out.surface = in.surface;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let shadow_factor = compute_shadow_factor(in.world_position, in.world_normal);
    let shaded = shade(in.world_position, in.world_normal, in.color.rgb, in.surface, 1.0, shadow_factor) + in.emissive;
    return vec4<f32>(shaded, in.color.a);
}
";

/// [`UNLIT_SHADER_WGSL`](crate::submission::UNLIT_SHADER_WGSL)'s lit counterpart: metallic-roughness-shaded flat
/// color. Used for lit materials with no resolvable albedo texture.
pub(crate) fn lit_shader_wgsl() -> String {
    format!(
        "{LIT_UNIFORMS_AND_SHADING_WGSL}{ENVIRONMENT_WGSL}{SHADOW_LOOKUP_WGSL}{LIT_COLORED_BODY_WGSL}"
    )
}

/// The lit textured fragment/vertex stage text spliced after the shared
/// uniforms and shading helpers.
const LIT_TEXTURED_BODY_WGSL: &str = r#"
@group(0) @binding(1)
var albedo_tex: texture_2d<f32>;
@group(0) @binding(2)
var albedo_sampler: sampler;
@group(0) @binding(3)
var shadow_map: texture_depth_2d;
@group(0) @binding(4)
var shadow_sampler: sampler_comparison;
// Roughness in green, metalness in blue (glTF layout); linear data.
@group(0) @binding(5)
var surface_tex: texture_2d<f32>;
// Tangent-space normal, occlusion in red, emitted colour (ADR 033).
@group(0) @binding(6)
var normal_tex: texture_2d<f32>;
@group(0) @binding(7)
var occlusion_tex: texture_2d<f32>;
@group(0) @binding(8)
var emissive_tex: texture_2d<f32>;
// The prefiltered environment and its sampler (ADR 035).
@group(0) @binding(9)
var env_map: texture_2d<f32>;
@group(0) @binding(10)
var env_sampler: sampler;

/// `world_normal` bent by the tangent-space `map_normal`. The tangent frame
/// comes from the screen-space derivatives of position and UV, so meshes
/// need no tangent attribute; the bitangent is `cross(n, t)`, which is
/// glTF's convention for a v axis that points down the image.
fn perturb_normal(n: vec3<f32>, world_pos: vec3<f32>, uv: vec2<f32>, map_normal: vec3<f32>) -> vec3<f32> {
    let dp1 = dpdx(world_pos);
    let dp2 = dpdy(world_pos);
    let duv1 = dpdx(uv);
    let duv2 = dpdy(uv);
    let det = duv1.x * duv2.y - duv2.x * duv1.y;
    if (abs(det) < 1e-12) {
        return n;
    }
    var t = (dp1 * duv2.y - dp2 * duv1.y) / det;
    t = t - n * dot(n, t);
    if (dot(t, t) < 1e-12) {
        return n;
    }
    t = normalize(t);
    let b = cross(n, t);
    return normalize(t * map_normal.x + b * map_normal.y + n * map_normal.z);
}

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
    @location(10) map_strength: vec2<f32>,
    @location(11) mapped_emissive: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) emissive: vec3<f32>,
    @location(5) surface: vec2<f32>,
    @location(6) map_strength: vec2<f32>,
    @location(7) mapped_emissive: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    let model = mat4x4<f32>(in.model_0, in.model_1, in.model_2, in.model_3);
    let world = model * vec4<f32>(in.position, 1.0);
    let world_normal = (model * vec4<f32>(in.normal, 0.0)).xyz;
    out.clip_position = u.view_proj * world;
    out.world_position = world.xyz;
    out.world_normal = world_normal;
    out.uv = in.uv;
    out.tint = in.color;
    out.emissive = in.emissive;
    out.surface = in.surface;
    out.map_strength = in.map_strength;
    out.mapped_emissive = in.mapped_emissive;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(albedo_tex, albedo_sampler, in.uv);
    let base_color = sampled.rgb * in.tint.rgb;
    let mapped = textureSample(surface_tex, albedo_sampler, in.uv);
    let surface = vec2<f32>(in.surface.x * mapped.g, in.surface.y * mapped.b);
    let tangent_normal = textureSample(normal_tex, albedo_sampler, in.uv).xyz * 2.0 - 1.0;
    let scaled = vec3<f32>(tangent_normal.xy * in.map_strength.x, tangent_normal.z);
    let world_normal = perturb_normal(normalize(in.world_normal), in.world_position, in.uv, normalize(scaled));
    let occlusion = mix(1.0, textureSample(occlusion_tex, albedo_sampler, in.uv).r, in.map_strength.y);
    let emitted = in.emissive + in.mapped_emissive * textureSample(emissive_tex, albedo_sampler, in.uv).rgb;
    let shadow_factor = compute_shadow_factor(in.world_position, world_normal);
    let shaded = shade(in.world_position, world_normal, base_color, surface, occlusion, shadow_factor) + emitted;
    return vec4<f32>(shaded, sampled.a * in.tint.a);
}
"#;

/// [`TEXTURED_SHADER_WGSL`](crate::submission::TEXTURED_SHADER_WGSL)'s lit counterpart.
pub(crate) fn lit_textured_shader_wgsl() -> String {
    format!(
        "{LIT_UNIFORMS_AND_SHADING_WGSL}{ENVIRONMENT_WGSL}{SHADOW_LOOKUP_WGSL}{LIT_TEXTURED_BODY_WGSL}"
    )
}

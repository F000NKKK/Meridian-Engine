//! The lit WGSL shaders and the light limit they are compiled for.

/// The uniform array's fixed capacity — see the module doc.
pub const MAX_LIGHTS: usize = 4;

/// Shared by every lit shader: the `Uniforms` struct layout (view-proj,
/// the shadow-casting light's own view-proj, camera position, hemisphere
/// ambient, light count/array, which light index casts the shadow) plus
/// the Blinn-Phong `shade` helper and the light-space projection helper
/// every shadow lookup shares. WGSL has no `#include`, so this text is
/// spliced verbatim into each lit shader constant below — keeping one
/// source of truth here rather than hand-duplicating it at each call
/// site. The shadow *map*/*sampler* bindings themselves are declared per
/// shader flavor below, not here — colored and textured shaders already
/// use different binding numbers for their own resources (the textured
/// shader's albedo texture/sampler occupy bindings 1/2), so the shadow
/// map/sampler can't share one fixed binding number across both; each
/// flavor's own `compute_shadow_factor` wraps its own declarations and
/// hands `shade` a plain `f32`, keeping this shared text binding-number-
/// agnostic.
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
    light_view_proj: mat4x4<f32>,
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

/// `world_pos` remapped into the shadow map's `[0,1]` UV space plus its
/// light-space depth in `z` — shared by every shader flavor's own
/// `compute_shadow_factor` (declared per-flavor below, since each names
/// its own `shadow_map`/`shadow_sampler` bindings). Orthographic
/// projection: `clip.w` is always `1.0`, so the divide is a no-op in
/// practice — kept for clarity, and so this still works unmodified if a
/// perspective shadow projection (e.g. for a point-light shadow) is ever
/// spliced in against the same `light_view_proj` slot.
fn light_space_uv_and_depth(world_pos: vec3<f32>) -> vec3<f32> {
    let clip = u.light_view_proj * vec4<f32>(world_pos, 1.0);
    let ndc = clip.xyz / clip.w;
    return vec3<f32>(ndc.x * 0.5 + 0.5, 1.0 - (ndc.y * 0.5 + 0.5), ndc.z);
}

fn shade(world_pos: vec3<f32>, world_normal: vec3<f32>, base_color: vec3<f32>, gloss: vec2<f32>, shadow_factor: f32) -> vec3<f32> {
    let n = normalize(world_normal);
    let view_dir = normalize(u.camera_pos.xyz - world_pos);
    var result = hemisphere_ambient(n) * base_color;

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
        let diffuse = max(dot(n, light_dir), 0.0);
        let half_dir = normalize(light_dir + view_dir);
        let specular = pow(max(dot(n, half_dir), 0.0), gloss.x) * gloss.y;
        let radiance = light.color_intensity.rgb * light.color_intensity.a * attenuation;
        result = result + (base_color * diffuse + vec3<f32>(specular, specular, specular)) * radiance;
    }
    return result;
}
"#;

/// The lit fragment/vertex stage text spliced after the shared
/// uniforms and shading helpers.
const LIT_COLORED_BODY_WGSL: &str = r#"

@group(0) @binding(1)
var shadow_map: texture_depth_2d;
@group(0) @binding(2)
var shadow_sampler: sampler_comparison;

// Slope-scaled bias: a surface nearly edge-on to the light (small
// `n_dot_l`) needs a much bigger depth push than one facing the light
// head-on, or it reads as "acne" — regularly-spaced false self-shadow
// stripes from the shadow map's own texel quantization (a pyramid's
// steep triangular faces were the concrete case that went essentially
// solid-black from this before the slope term existed: at a grazing
// angle, a constant bias big enough to fix the steepest face was still
// far too small there, and the acne covered the whole face). The base
// term alone still matters for a face that's *exactly* perpendicular to
// the light (n_dot_l = 1), where the slope term contributes nothing.
fn compute_shadow_factor(world_pos: vec3<f32>, world_normal: vec3<f32>) -> f32 {
    let lsuv = light_space_uv_and_depth(world_pos);
    if (lsuv.x < 0.0 || lsuv.x > 1.0 || lsuv.y < 0.0 || lsuv.y > 1.0) {
        return 1.0;
    }
    let n_dot_l = max(dot(normalize(world_normal), normalize(-u.light_direction.xyz)), 0.0);
    let bias = max(0.0008, 0.006 * (1.0 - n_dot_l));
    return textureSampleCompare(shadow_map, shadow_sampler, lsuv.xy, lsuv.z - bias);
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) emissive: vec3<f32>,
    @location(4) gloss: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) emissive: vec3<f32>,
    @location(4) gloss: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u.view_proj * vec4<f32>(in.position, 1.0);
    out.world_position = in.position;
    out.world_normal = in.normal;
    out.color = in.color;
    out.emissive = in.emissive;
    out.gloss = in.gloss;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let shadow_factor = compute_shadow_factor(in.world_position, in.world_normal);
    let shaded = shade(in.world_position, in.world_normal, in.color.rgb, in.gloss, shadow_factor) + in.emissive;
    return vec4<f32>(shaded, in.color.a);
}
"#;

/// [`UNLIT_SHADER_WGSL`](crate::submission::UNLIT_SHADER_WGSL)'s lit counterpart: Blinn-Phong-shaded flat
/// color. Used for lit materials with no resolvable albedo texture.
pub(crate) fn lit_shader_wgsl() -> String {
    format!("{LIT_UNIFORMS_AND_SHADING_WGSL}{LIT_COLORED_BODY_WGSL}")
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

// See the colored-lit shader's identical `compute_shadow_factor` for
// why the bias is slope-scaled, not a flat constant.
fn compute_shadow_factor(world_pos: vec3<f32>, world_normal: vec3<f32>) -> f32 {
    let lsuv = light_space_uv_and_depth(world_pos);
    if (lsuv.x < 0.0 || lsuv.x > 1.0 || lsuv.y < 0.0 || lsuv.y > 1.0) {
        return 1.0;
    }
    let n_dot_l = max(dot(normalize(world_normal), normalize(-u.light_direction.xyz)), 0.0);
    let bias = max(0.0008, 0.006 * (1.0 - n_dot_l));
    return textureSampleCompare(shadow_map, shadow_sampler, lsuv.xy, lsuv.z - bias);
}

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) emissive: vec3<f32>,
    @location(5) gloss: vec2<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) emissive: vec3<f32>,
    @location(5) gloss: vec2<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u.view_proj * vec4<f32>(in.position, 1.0);
    out.world_position = in.position;
    out.world_normal = in.normal;
    out.uv = in.uv;
    out.tint = in.tint;
    out.emissive = in.emissive;
    out.gloss = in.gloss;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(albedo_tex, albedo_sampler, in.uv);
    let base_color = sampled.rgb * in.tint.rgb;
    let shadow_factor = compute_shadow_factor(in.world_position, in.world_normal);
    let shaded = shade(in.world_position, in.world_normal, base_color, in.gloss, shadow_factor) + in.emissive;
    return vec4<f32>(shaded, sampled.a * in.tint.a);
}
"#;

/// [`TEXTURED_SHADER_WGSL`](crate::submission::TEXTURED_SHADER_WGSL)'s lit counterpart.
pub(crate) fn lit_textured_shader_wgsl() -> String {
    format!("{LIT_UNIFORMS_AND_SHADING_WGSL}{LIT_TEXTURED_BODY_WGSL}")
}

//! The submission bridge: turns a culled [`Scene3D`] into real GPU draws
//! through `graphics-driver` — step 2 of docs/graphics-design.md's scene
//! vocabulary implementation order (texture-sampled materials), plus
//! step 3 (Blinn-Phong lighting). This is the one place in
//! `graphics-core` allowed to touch `graphics-driver` objects; every
//! other type in this crate stays driver-free. [`crate::bloom`] builds
//! on top of the emissive channel this module bakes per-vertex.
//!
//! **Mesh/material/texture identity is handle-based, never a raw index
//! or a driver object** ([`MeshRegistry`]/[`MaterialRegistry`]/
//! [`TextureRegistry`] wrap `memory-core::ResourcePool`, resolved by
//! [`MeshHandle`]/[`MaterialHandle`]/[`TextureHandle`] — see ADR 002).
//!
//! **Four pipelines, chosen per renderable at bake time:** textured
//! vs. flat-color (whether `Material::albedo` resolves in the
//! [`TextureRegistry`]) crossed with lit vs. `Material::unlit`.
//!
//! Lit pipelines run Blinn-Phong forward shading against
//! `Scene3D::lights` + `Scene3D::ambient` — a fixed-shininess,
//! no-shadows approximation (real material roughness/specular control
//! and shadow mapping are future work, not this step); unlit pipelines
//! skip lighting entirely, for world-space UI and anything meant to
//! read at a constant brightness. `Material::emissive` is added on top
//! in both cases and baked into every vertex regardless of family — see
//! `bake_draw_buffers` — so a mesh can glow (feed a bloom pass)
//! whether or not it's lit.
//!
//! **No per-instance uniform, no GPU-side instancing yet.** A render
//! pass here has exactly one shared uniform buffer (view-projection,
//! camera position, ambient, lights) per view — there is no dynamic-
//! offset or per-draw uniform mechanism in `graphics-driver` today, and
//! writing a *shared* uniform once per draw would race (every draw only
//! executes once the whole command buffer is submitted, by which point
//! the last write wins for *all* of them — the same footgun the
//! soft-body examples' buffer-lifetime comments call out). So each
//! [`Renderable3D`]'s world-space position/normal, its UV (if textured)
//! and its material's tint/emissive are baked into a fresh vertex buffer
//! at [`SceneRenderer::prepare`] time, once per instance, per frame —
//! correct today, and exactly the CPU-side cost docs/graphics-design.md's
//! "GPU-driven rendering" section already names as the long-term thing
//! to fix via instancing/indirect draw. Per-textured-draw bind groups
//! are similarly rebuilt every frame rather than cached per texture —
//! the same "correct now, batch later" trade-off.
//!
//! **Lights are capped at [`MAX_LIGHTS`]** (a fixed-size WGSL uniform
//! array, not a dynamically-sized storage buffer — keeping the uniform
//! shape simple until a scene actually needs more): extra lights beyond
//! the cap are dropped, logged once per [`SceneRenderer::prepare`] call
//! via `meridian_foundation::log_warn!`, not silently ignored.

use meridian_gac_core::{Motor3, Projection, Vec3};
use meridian_graphics_driver::{
    BindGroup, Buffer, BufferUsage, CommandBuffer, Device, RenderPass as DriverRenderPass,
    RenderPipeline, Sampler, Surface, Texture, VertexAttributeDesc, VertexFormat, VertexLayout,
};
use meridian_memory_core::ResourcePool;

use crate::scene::{Light, Material, Renderable3D, Scene3D};
use crate::{Camera, MaterialHandle, MeshHandle, TextureHandle, look_at_rotor};

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
/// `ambient_sky` (facing straight up) — see [`crate::scene::Scene3D`]'s
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

fn shade(world_pos: vec3<f32>, world_normal: vec3<f32>, base_color: vec3<f32>, shadow_factor: f32) -> vec3<f32> {
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
        let specular = pow(max(dot(n, half_dir), 0.0), 32.0) * 0.3;
        let radiance = light.color_intensity.rgb * light.color_intensity.a * attenuation;
        result = result + (base_color * diffuse + vec3<f32>(specular, specular, specular)) * radiance;
    }
    return result;
}
"#;

/// A flat, unlit pipeline: `view_proj * vec4(position, 1)`, output color
/// is the per-vertex color + emissive baked in at
/// [`SceneRenderer::prepare`] time (see the module doc for why it's
/// per-vertex, not a material uniform). Used for `Material::unlit`
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
    @location(2) color: vec4<f32>,
    @location(3) emissive: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) emissive: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = u.view_proj * vec4<f32>(in.position, 1.0);
    out.color = in.color;
    out.emissive = in.emissive;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(in.color.rgb + in.emissive, in.color.a);
}
"#;

/// [`UNLIT_SHADER_WGSL`]'s lit counterpart: Blinn-Phong-shaded flat
/// color. Used for lit materials with no resolvable albedo texture.
pub fn lit_shader_wgsl() -> String {
    format!(
        r#"
{LIT_UNIFORMS_AND_SHADING_WGSL}

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
fn compute_shadow_factor(world_pos: vec3<f32>, world_normal: vec3<f32>) -> f32 {{
    let lsuv = light_space_uv_and_depth(world_pos);
    if (lsuv.x < 0.0 || lsuv.x > 1.0 || lsuv.y < 0.0 || lsuv.y > 1.0) {{
        return 1.0;
    }}
    let n_dot_l = max(dot(normalize(world_normal), normalize(-u.light_direction.xyz)), 0.0);
    let bias = max(0.0008, 0.006 * (1.0 - n_dot_l));
    return textureSampleCompare(shadow_map, shadow_sampler, lsuv.xy, lsuv.z - bias);
}}

struct VertexInput {{
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) emissive: vec3<f32>,
}};

struct VertexOutput {{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) color: vec4<f32>,
    @location(3) emissive: vec3<f32>,
}};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {{
    var out: VertexOutput;
    out.clip_position = u.view_proj * vec4<f32>(in.position, 1.0);
    out.world_position = in.position;
    out.world_normal = in.normal;
    out.color = in.color;
    out.emissive = in.emissive;
    return out;
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    let shadow_factor = compute_shadow_factor(in.world_position, in.world_normal);
    let shaded = shade(in.world_position, in.world_normal, in.color.rgb, shadow_factor) + in.emissive;
    return vec4<f32>(shaded, in.color.a);
}}
"#
    )
}

/// An unlit, textured pipeline: samples `albedo_tex` and multiplies by
/// the per-vertex tint (the material's `base_color_factor`), plus
/// per-vertex emissive — baked in at [`SceneRenderer::prepare`] time
/// (see the module doc for why tint/emissive travel per-vertex rather
/// than through a second uniform). Used for `Material::unlit` materials
/// whose albedo resolves in a [`TextureRegistry`].
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
    @location(3) tint: vec4<f32>,
    @location(4) emissive: vec3<f32>,
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
    out.clip_position = u.view_proj * vec4<f32>(in.position, 1.0);
    out.uv = in.uv;
    out.tint = in.tint;
    out.emissive = in.emissive;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sampled = textureSample(albedo_tex, albedo_sampler, in.uv);
    return vec4<f32>(sampled.rgb * in.tint.rgb + in.emissive, sampled.a * in.tint.a);
}
"#;

/// [`TEXTURED_SHADER_WGSL`]'s lit counterpart.
pub fn lit_textured_shader_wgsl() -> String {
    format!(
        r#"
{LIT_UNIFORMS_AND_SHADING_WGSL}
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
fn compute_shadow_factor(world_pos: vec3<f32>, world_normal: vec3<f32>) -> f32 {{
    let lsuv = light_space_uv_and_depth(world_pos);
    if (lsuv.x < 0.0 || lsuv.x > 1.0 || lsuv.y < 0.0 || lsuv.y > 1.0) {{
        return 1.0;
    }}
    let n_dot_l = max(dot(normalize(world_normal), normalize(-u.light_direction.xyz)), 0.0);
    let bias = max(0.0008, 0.006 * (1.0 - n_dot_l));
    return textureSampleCompare(shadow_map, shadow_sampler, lsuv.xy, lsuv.z - bias);
}}

struct VertexInput {{
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) emissive: vec3<f32>,
}};

struct VertexOutput {{
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec3<f32>,
    @location(1) world_normal: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) tint: vec4<f32>,
    @location(4) emissive: vec3<f32>,
}};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {{
    var out: VertexOutput;
    out.clip_position = u.view_proj * vec4<f32>(in.position, 1.0);
    out.world_position = in.position;
    out.world_normal = in.normal;
    out.uv = in.uv;
    out.tint = in.tint;
    out.emissive = in.emissive;
    return out;
}}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {{
    let sampled = textureSample(albedo_tex, albedo_sampler, in.uv);
    let base_color = sampled.rgb * in.tint.rgb;
    let shadow_factor = compute_shadow_factor(in.world_position, in.world_normal);
    let shaded = shade(in.world_position, in.world_normal, base_color, shadow_factor) + in.emissive;
    return vec4<f32>(shaded, sampled.a * in.tint.a);
}}
"#
    )
}

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

/// [`UNLIT_SHADER_WGSL`]/[`lit_shader_wgsl`]'s shared vertex layout:
/// position, normal, color, emissive — tightly packed.
fn colored_vertex_layout() -> VertexLayout {
    VertexLayout {
        stride: 52, // 3 + 3 + 4 + 3 = 13 floats
        attributes: vec![
            VertexAttributeDesc {
                location: 0,
                format: VertexFormat::Float32x3,
                offset: 0,
            },
            VertexAttributeDesc {
                location: 1,
                format: VertexFormat::Float32x3,
                offset: 12,
            },
            VertexAttributeDesc {
                location: 2,
                format: VertexFormat::Float32x4,
                offset: 24,
            },
            VertexAttributeDesc {
                location: 3,
                format: VertexFormat::Float32x3,
                offset: 40,
            },
        ],
    }
}

/// [`TEXTURED_SHADER_WGSL`]/[`lit_textured_shader_wgsl`]'s shared vertex
/// layout: position, normal, uv, tint, emissive — tightly packed.
fn textured_vertex_layout() -> VertexLayout {
    VertexLayout {
        stride: 60, // 3 + 3 + 2 + 4 + 3 = 15 floats
        attributes: vec![
            VertexAttributeDesc {
                location: 0,
                format: VertexFormat::Float32x3,
                offset: 0,
            },
            VertexAttributeDesc {
                location: 1,
                format: VertexFormat::Float32x3,
                offset: 12,
            },
            VertexAttributeDesc {
                location: 2,
                format: VertexFormat::Float32x2,
                offset: 24,
            },
            VertexAttributeDesc {
                location: 3,
                format: VertexFormat::Float32x4,
                offset: 32,
            },
            VertexAttributeDesc {
                location: 4,
                format: VertexFormat::Float32x3,
                offset: 48,
            },
        ],
    }
}

/// Reinterprets [`colored_vertex_layout`]'s buffer bytes as just
/// position + emissive (same stride, two attributes) — what
/// [`EMISSIVE_EXTRACT_SHADER_WGSL`] reads for a colored-family draw.
pub(crate) fn emissive_from_colored_layout() -> VertexLayout {
    VertexLayout {
        stride: 52,
        attributes: vec![
            VertexAttributeDesc {
                location: 0,
                format: VertexFormat::Float32x3,
                offset: 0,
            },
            VertexAttributeDesc {
                location: 1,
                format: VertexFormat::Float32x3,
                offset: 40,
            },
        ],
    }
}

/// [`emissive_from_colored_layout`]'s textured-family counterpart.
pub(crate) fn emissive_from_textured_layout() -> VertexLayout {
    VertexLayout {
        stride: 60,
        attributes: vec![
            VertexAttributeDesc {
                location: 0,
                format: VertexFormat::Float32x3,
                offset: 0,
            },
            VertexAttributeDesc {
                location: 1,
                format: VertexFormat::Float32x3,
                offset: 48,
            },
        ],
    }
}

/// Position-only reinterpretation of [`colored_vertex_layout`]'s buffer
/// bytes — what the shadow depth pass reads for a colored-family draw
/// (see [`SHADOW_DEPTH_SHADER_WGSL`]): the depth pass only needs
/// clip-space position, so normal/color/emissive are never touched,
/// even though they're still physically present in the same buffer.
pub(crate) fn shadow_from_colored_layout() -> VertexLayout {
    VertexLayout {
        stride: 52,
        attributes: vec![VertexAttributeDesc {
            location: 0,
            format: VertexFormat::Float32x3,
            offset: 0,
        }],
    }
}

/// `shadow_from_colored_layout`'s textured-family counterpart.
pub(crate) fn shadow_from_textured_layout() -> VertexLayout {
    VertexLayout {
        stride: 60,
        attributes: vec![VertexAttributeDesc {
            location: 0,
            format: VertexFormat::Float32x3,
            offset: 0,
        }],
    }
}

/// The shadow map's own depth-only pass: transforms each vertex by the
/// shadow-casting light's own view-projection matrix (not the camera's)
/// and writes nothing but depth — no fragment stage at all (see
/// `graphics-driver::Device::create_shadow_pipeline`, which builds this
/// shader with `fragment: None`). Reused unmodified for both colored and
/// textured draws via `shadow_from_colored_layout`/
/// `shadow_from_textured_layout` — this shader only ever reads
/// `@location(0) position`, so the rest of either family's vertex bytes
/// (already baked into the same buffer the main pass uses — see
/// [`SceneRenderer::render_shadow_pass`]) are simply never touched.
pub const SHADOW_DEPTH_SHADER_WGSL: &str = r#"
struct ShadowUniforms {
    light_view_proj: mat4x4<f32>,
};

@group(0) @binding(0)
var<uniform> u: ShadowUniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> @builtin(position) vec4<f32> {
    return u.light_view_proj * vec4<f32>(in.position, 1.0);
}
"#;

/// Fixed half-extent (world units) of the directional shadow map's
/// orthographic volume, centered on the world origin — a disclosed
/// simplification: a real engine fits this box to the camera frustum
/// each frame (a "shadow cascade"); this workspace's scenes are small
/// and roughly origin-centered, so one fixed, generous box covers both
/// `physic_figures` (floor half-extent 14) and `magic_figures` (orbit
/// radius 3.2, floor half-extent 14) without per-frame recomputation.
pub const SHADOW_VOLUME_HALF_EXTENT: f32 = 20.0;

/// Builds the [`Camera`] a directional light "sees" its shadow-casting
/// geometry through: an orthographic box of side
/// `2 * SHADOW_VOLUME_HALF_EXTENT` centered on `scene_center`, looking
/// along `light_direction`. Reused as an ordinary [`Camera`] (see that
/// type's own `view_projection_matrix`) so the shadow map's light-space
/// matrix goes through the exact same, independently-tested view/
/// projection math every other camera in this workspace does — not a
/// hand-rolled parallel matrix path.
pub fn directional_shadow_camera(light_direction: Vec3, scene_center: Vec3) -> Camera {
    let forward = light_direction.normalize();
    // The light "camera" sits behind the scene, looking toward it along
    // `forward` — far enough back that `scene_center` sits comfortably
    // inside the ortho box's near/far range for any `forward` direction.
    let eye = scene_center - forward * (SHADOW_VOLUME_HALF_EXTENT * 2.0);
    let rotor = look_at_rotor(eye, scene_center);
    let h = SHADOW_VOLUME_HALF_EXTENT;
    Camera {
        frame: Motor3::from_rotation_translation(rotor, eye),
        projection: Projection::orthographic(-h, h, -h, h, 0.1, h * 4.0),
    }
}

fn mat4_to_bytes(m: [[f32; 4]; 4]) -> [u8; 64] {
    let mut bytes = [0u8; 64];
    let mut offset = 0;
    for column in m {
        for component in column {
            bytes[offset..offset + 4].copy_from_slice(&component.to_le_bytes());
            offset += 4;
        }
    }
    bytes
}

fn vec3_to_padded_bytes(v: [f32; 3]) -> [u8; 16] {
    let mut bytes = [0u8; 16];
    for (i, component) in v.iter().enumerate() {
        bytes[i * 4..i * 4 + 4].copy_from_slice(&component.to_le_bytes());
    }
    bytes
}

/// Serializes `camera`/`light_view_proj`/`ambient_ground`/`ambient_sky`/
/// `shadow_caster`/`lights` into the lit shaders' shared `Uniforms`
/// layout (see [`LIT_UNIFORMS_AND_SHADING_WGSL`]): `view_proj` (64
/// bytes), `light_view_proj` (64), `camera_pos` (16), `ambient_ground`
/// (16), `ambient_sky` (16), `light_count` (16), `shadow_caster` (16),
/// then [`MAX_LIGHTS`] `LightData` entries (48 bytes each) — 400 bytes
/// total. Lights beyond the cap are dropped (logged once by the caller,
/// not here — see [`SceneRenderer::prepare`]). `shadow_caster` is the
/// index into `lights` of the shadow-casting light (see
/// [`SceneRenderer::render_shadow_pass`]), or `u32::MAX` if none —
/// `shade`'s shadow lookup only ever matches a real light index, never
/// `u32::MAX`, so "no shadow-casting light" safely degrades to "no
/// light gets a shadow factor applied," not an out-of-bounds index.
fn lit_uniform_bytes(
    camera: &Camera,
    light_view_proj: [[f32; 4]; 4],
    ambient_ground: [f32; 3],
    ambient_sky: [f32; 3],
    shadow_caster: u32,
    lights: &[Light],
) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(400);
    bytes.extend_from_slice(&mat4_to_bytes(camera.view_projection_matrix()));
    bytes.extend_from_slice(&mat4_to_bytes(light_view_proj));
    let camera_pos = camera.frame.transform_point(Vec3::ZERO);
    bytes.extend_from_slice(&vec3_to_padded_bytes([
        camera_pos.x,
        camera_pos.y,
        camera_pos.z,
    ]));
    bytes.extend_from_slice(&vec3_to_padded_bytes(ambient_ground));
    bytes.extend_from_slice(&vec3_to_padded_bytes(ambient_sky));
    let count = lights.len().min(MAX_LIGHTS) as u32;
    bytes.extend_from_slice(&count.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 12]); // pad light_count to a vec4<u32>
    bytes.extend_from_slice(&shadow_caster.to_le_bytes());
    bytes.extend_from_slice(&[0u8; 12]); // pad shadow_caster to a vec4<u32>

    for i in 0..MAX_LIGHTS {
        match lights.get(i) {
            Some(Light::Directional {
                direction,
                color,
                intensity,
            }) => {
                let dir: Vec3 = direction.transform_vector(Vec3::X);
                bytes.extend_from_slice(&dir.x.to_le_bytes());
                bytes.extend_from_slice(&dir.y.to_le_bytes());
                bytes.extend_from_slice(&dir.z.to_le_bytes());
                bytes.extend_from_slice(&0.0f32.to_le_bytes()); // kind = directional
                bytes.extend_from_slice(&color[0].to_le_bytes());
                bytes.extend_from_slice(&color[1].to_le_bytes());
                bytes.extend_from_slice(&color[2].to_le_bytes());
                bytes.extend_from_slice(&intensity.to_le_bytes());
                bytes.extend_from_slice(&[0u8; 16]); // range_pad, unused
            }
            Some(Light::Point {
                position,
                color,
                intensity,
                range,
            }) => {
                let pos: Vec3 = position.transform_point(Vec3::ZERO);
                bytes.extend_from_slice(&pos.x.to_le_bytes());
                bytes.extend_from_slice(&pos.y.to_le_bytes());
                bytes.extend_from_slice(&pos.z.to_le_bytes());
                bytes.extend_from_slice(&1.0f32.to_le_bytes()); // kind = point
                bytes.extend_from_slice(&color[0].to_le_bytes());
                bytes.extend_from_slice(&color[1].to_le_bytes());
                bytes.extend_from_slice(&color[2].to_le_bytes());
                bytes.extend_from_slice(&intensity.to_le_bytes());
                bytes.extend_from_slice(&range.to_le_bytes());
                bytes.extend_from_slice(&[0u8; 12]);
            }
            None => bytes.extend_from_slice(&[0u8; 48]),
        }
    }
    bytes
}

/// The CPU-side source a [`MeshHandle`] resolves to: local-space
/// positions, per-vertex normals and UVs, and triangle indices, decoded
/// elsewhere (typically `asset-core::ObjDecoder`/glTF once that lands)
/// and registered here once. `normals.len()`/`uvs.len()` must equal
/// `positions.len()`, `indices.len()` must be a multiple of 3 (triangle
/// list), and `positions.len()` must fit in `u16` — `graphics-driver`
/// only supports `u16` index buffers today (see
/// `meridian_graphics_driver::RenderPass::set_index_buffer_u16`'s own
/// doc comment); meshes beyond 65535 vertices are a future `u32`-index
/// follow-up, not something this bridge silently mishandles (see
/// [`MeshRegistry::register`]).
#[derive(Debug, Clone)]
pub struct MeshSource {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub indices: Vec<u32>,
}

/// Why [`MeshRegistry::register`] refused a [`MeshSource`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeshRegistryError {
    /// More vertices than a `u16` index can address (see [`MeshSource`]'s
    /// doc comment).
    TooManyVertices { count: usize },
    /// `indices.len()` isn't a multiple of 3 — not a triangle list.
    NotATriangleList { index_count: usize },
    /// `uvs.len()` doesn't match `positions.len()` — every vertex needs
    /// exactly one UV, even on meshes only ever drawn unlit/untextured
    /// (the registry doesn't know a mesh's eventual material up front).
    UvCountMismatch { positions: usize, uvs: usize },
    /// `normals.len()` doesn't match `positions.len()`.
    NormalCountMismatch { positions: usize, normals: usize },
}

impl core::fmt::Display for MeshRegistryError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            MeshRegistryError::TooManyVertices { count } => {
                write!(
                    f,
                    "mesh has {count} vertices, beyond u16 index range (65535)"
                )
            }
            MeshRegistryError::NotATriangleList { index_count } => {
                write!(f, "index count {index_count} is not a multiple of 3")
            }
            MeshRegistryError::UvCountMismatch { positions, uvs } => {
                write!(f, "{positions} positions but {uvs} uvs — must match")
            }
            MeshRegistryError::NormalCountMismatch { positions, normals } => {
                write!(
                    f,
                    "{positions} positions but {normals} normals — must match"
                )
            }
        }
    }
}

impl std::error::Error for MeshRegistryError {}

impl meridian_foundation::EngineError for MeshRegistryError {}

/// Handle-addressed storage for [`MeshSource`]s — the CPU-side registry
/// [`Renderable3D::mesh`] resolves against. Owns no GPU objects; GPU
/// buffers are built fresh per instance in [`SceneRenderer::prepare`]
/// (see the module doc).
#[derive(Debug, Default)]
pub struct MeshRegistry {
    sources: ResourcePool<MeshSource>,
}

impl MeshRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, source: MeshSource) -> Result<MeshHandle, MeshRegistryError> {
        if source.positions.len() > u16::MAX as usize {
            return Err(MeshRegistryError::TooManyVertices {
                count: source.positions.len(),
            });
        }
        if !source.indices.len().is_multiple_of(3) {
            return Err(MeshRegistryError::NotATriangleList {
                index_count: source.indices.len(),
            });
        }
        if source.uvs.len() != source.positions.len() {
            return Err(MeshRegistryError::UvCountMismatch {
                positions: source.positions.len(),
                uvs: source.uvs.len(),
            });
        }
        if source.normals.len() != source.positions.len() {
            return Err(MeshRegistryError::NormalCountMismatch {
                positions: source.positions.len(),
                normals: source.normals.len(),
            });
        }
        Ok(MeshHandle::new(self.sources.insert(source)))
    }

    pub fn unregister(&mut self, handle: MeshHandle) -> Option<MeshSource> {
        self.sources.remove(handle.handle)
    }

    pub fn get(&self, handle: MeshHandle) -> Option<&MeshSource> {
        self.sources.get(handle.handle)
    }
}

/// Handle-addressed storage for [`Material`]s. Materials are plain data
/// (see `scene::Material`'s own doc comment); this registry exists so
/// [`Renderable3D::material`] is a stable handle rather than an inline
/// value, matching every other resource identity in the workspace.
#[derive(Debug, Default)]
pub struct MaterialRegistry {
    materials: ResourcePool<Material>,
}

impl MaterialRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, material: Material) -> MaterialHandle {
        MaterialHandle::new(self.materials.insert(material))
    }

    pub fn unregister(&mut self, handle: MaterialHandle) -> Option<Material> {
        self.materials.remove(handle.handle)
    }

    pub fn get(&self, handle: MaterialHandle) -> Option<&Material> {
        self.materials.get(handle.handle)
    }
}

/// GPU-resident textures, handle-addressed by [`TextureHandle`] — the
/// registry a [`Material::albedo`] resolves against. Upload happens once
/// via [`upload`](Self::upload), from an already-decoded
/// `asset-core::ImageData` (RGBA8) — this bridge does not decode image
/// bytes itself, matching the crate's existing "decoding belongs to
/// `asset-core`" boundary.
#[derive(Debug, Default)]
pub struct TextureRegistry {
    textures: ResourcePool<Texture>,
}

impl TextureRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Uploads `image`'s pixels to a new GPU texture and returns its
    /// handle.
    pub fn upload(
        &mut self,
        device: &Device,
        image: &meridian_asset_core::ImageData,
    ) -> TextureHandle {
        let texture = device.create_texture_2d(image.width, image.height);
        device.write_texture(&texture, &image.pixels);
        TextureHandle::new(self.textures.insert(texture))
    }

    pub fn unregister(&mut self, handle: TextureHandle) -> Option<Texture> {
        self.textures.remove(handle.handle)
    }

    pub fn get(&self, handle: TextureHandle) -> Option<&Texture> {
        self.textures.get(handle.handle)
    }
}

/// Which family (colored vs. textured) one [`DrawBuffers`] entry belongs
/// to, and the per-draw state its main pipeline needs beyond the shared
/// vertex/index buffers.
pub(crate) enum DrawKind {
    Colored,
    /// A fresh bind group naming this draw's specific texture — see the
    /// module doc's "no per-texture bind-group caching yet" note.
    Textured {
        bind_group: BindGroup,
    },
}

/// One renderable's baked-for-this-frame GPU state — built by
/// [`SceneRenderer::prepare`] *before* the render pass opens, kept alive
/// by the caller until [`SceneRenderer::draw`] (and the pass itself) is
/// done with them. This split exists because `RenderPass::set_vertex_buffer`
/// borrows its buffer for the pass's lifetime — the same reason
/// `soft_body_rubber_balls`'s own `ball_buffers` are built up front (see
/// that example's module doc). [`crate::bloom`] redraws the same
/// `vertex_buffer`/`index_buffer` through its own emissive-extraction
/// pipeline, so both stay `pub(crate)`.
pub struct DrawBuffers {
    pub(crate) vertex_buffer: Buffer,
    pub(crate) index_buffer: Buffer,
    pub(crate) index_count: u32,
    lit: bool,
    pub(crate) kind: DrawKind,
}

/// Transforms `source`'s local-space positions/normals by `frame`, bakes
/// in `material`'s tint/emissive (and UVs, for the textured path), and
/// uploads the result — the flat/unlit-vertex-layout path when `albedo`
/// is `None`, the textured one (with a bind group naming `albedo`)
/// otherwise. `lit` only affects which pipeline [`SceneRenderer::draw`]
/// selects; the baked bytes are identical either way (see the module
/// doc: both lit and unlit pipelines within a family share one vertex
/// layout).
fn bake_draw_buffers(
    device: &Device,
    renderer: &SceneRenderer,
    source: &MeshSource,
    material: &Material,
    frame: &Motor3,
    albedo: Option<&Texture>,
) -> DrawBuffers {
    let index_bytes: Vec<u8> = source
        .indices
        .iter()
        .flat_map(|&i| (i as u16).to_le_bytes())
        .collect();
    let index_buffer = device.create_buffer(index_bytes.len(), BufferUsage::Index);
    device.write_buffer(&index_buffer, &index_bytes);
    let index_count = source.indices.len() as u32;
    let lit = !material.unlit;

    match albedo {
        Some(texture) => {
            let mut vertex_bytes = Vec::with_capacity(source.positions.len() * 60);
            for ((&[x, y, z], &[nx, ny, nz]), &[u, v]) in source
                .positions
                .iter()
                .zip(&source.normals)
                .zip(&source.uvs)
            {
                let world = frame.transform_point(Vec3::new(x, y, z));
                let normal = frame.transform_vector(Vec3::new(nx, ny, nz));
                vertex_bytes.extend_from_slice(&world.x.to_le_bytes());
                vertex_bytes.extend_from_slice(&world.y.to_le_bytes());
                vertex_bytes.extend_from_slice(&world.z.to_le_bytes());
                vertex_bytes.extend_from_slice(&normal.x.to_le_bytes());
                vertex_bytes.extend_from_slice(&normal.y.to_le_bytes());
                vertex_bytes.extend_from_slice(&normal.z.to_le_bytes());
                vertex_bytes.extend_from_slice(&u.to_le_bytes());
                vertex_bytes.extend_from_slice(&v.to_le_bytes());
                for component in material.base_color_factor {
                    vertex_bytes.extend_from_slice(&component.to_le_bytes());
                }
                for component in material.emissive {
                    vertex_bytes.extend_from_slice(&component.to_le_bytes());
                }
            }
            let vertex_buffer = device.create_buffer(vertex_bytes.len(), BufferUsage::Vertex);
            device.write_buffer(&vertex_buffer, &vertex_bytes);
            let pipeline = if lit {
                &renderer.textured_lit_pipeline
            } else {
                &renderer.textured_unlit_pipeline
            };
            // Lit textured draws need the shadow map/sampler in their
            // bind group too (see `lit_textured_shader_wgsl`'s bindings
            // 3/4); unlit ignores lighting entirely, so its bind group
            // stays the plain 3-binding shape.
            let bind_group = if lit {
                device.create_textured_shadow_bind_group(
                    pipeline,
                    &renderer.uniform_buffer,
                    texture,
                    &renderer.sampler,
                    &renderer.shadow_map,
                    &renderer.shadow_sampler,
                )
            } else {
                device.create_textured_bind_group(
                    pipeline,
                    &renderer.uniform_buffer,
                    texture,
                    &renderer.sampler,
                )
            };
            DrawBuffers {
                vertex_buffer,
                index_buffer,
                index_count,
                lit,
                kind: DrawKind::Textured { bind_group },
            }
        }
        None => {
            let mut vertex_bytes = Vec::with_capacity(source.positions.len() * 52);
            for (&[x, y, z], &[nx, ny, nz]) in source.positions.iter().zip(&source.normals) {
                let world = frame.transform_point(Vec3::new(x, y, z));
                let normal = frame.transform_vector(Vec3::new(nx, ny, nz));
                vertex_bytes.extend_from_slice(&world.x.to_le_bytes());
                vertex_bytes.extend_from_slice(&world.y.to_le_bytes());
                vertex_bytes.extend_from_slice(&world.z.to_le_bytes());
                vertex_bytes.extend_from_slice(&normal.x.to_le_bytes());
                vertex_bytes.extend_from_slice(&normal.y.to_le_bytes());
                vertex_bytes.extend_from_slice(&normal.z.to_le_bytes());
                for component in material.base_color_factor {
                    vertex_bytes.extend_from_slice(&component.to_le_bytes());
                }
                for component in material.emissive {
                    vertex_bytes.extend_from_slice(&component.to_le_bytes());
                }
            }
            let vertex_buffer = device.create_buffer(vertex_bytes.len(), BufferUsage::Vertex);
            device.write_buffer(&vertex_buffer, &vertex_bytes);
            DrawBuffers {
                vertex_buffer,
                index_buffer,
                index_count,
                lit,
                kind: DrawKind::Colored,
            }
        }
    }
}

/// Owns all four main pipelines (colored/textured × lit/unlit) plus the
/// the shared view/lighting uniform, and the sampler for rendering a
/// [`Scene3D`] view. One instance per view (a UI overlay's `Scene2D` is
/// a separate, future-work sibling — see docs/graphics-design.md);
/// building it needs a real GPU device and a surface to match color/
/// depth formats against. The emissive-extraction pipelines
/// [`crate::bloom::BloomPass`] draws through are *not* owned here — they
/// render into `BloomPass`'s own offscreen texture, whose format is
/// `BloomPass`'s to pick, not something this type should guess at (see
/// `bloom.rs`'s module doc and `uniform_buffer`(Self::uniform_buffer)).
pub struct SceneRenderer {
    colored_unlit_pipeline: RenderPipeline,
    colored_unlit_bind_group: BindGroup,
    colored_lit_pipeline: RenderPipeline,
    colored_lit_bind_group: BindGroup,
    textured_unlit_pipeline: RenderPipeline,
    textured_lit_pipeline: RenderPipeline,
    uniform_buffer: Buffer,
    sampler: Sampler,
    /// A `Depth32Float` texture, both a render target (the shadow depth
    /// pass writes into it) and sampleable (the lit shaders' shadow
    /// lookup reads it) — see
    /// `graphics-driver::Device::create_shadow_map_texture`'s own doc
    /// comment for why [`Texture`] rather than the plain
    /// render-target-only `DepthTexture`.
    shadow_map: Texture,
    shadow_sampler: Sampler,
    shadow_uniform_buffer: Buffer,
    // Two bind groups, not one: `wgpu`'s auto-derived (`layout: None`)
    // pipeline layout is unique *per pipeline object*, even for two
    // pipelines built from identical shader text — a bind group built
    // against one pipeline's layout fails validation against the
    // other's, so `shadow_colored_pipeline`/`shadow_textured_pipeline`
    // each need their own, both wrapping the same underlying
    // `shadow_uniform_buffer`.
    shadow_colored_bind_group: BindGroup,
    shadow_textured_bind_group: BindGroup,
    shadow_colored_pipeline: RenderPipeline,
    shadow_textured_pipeline: RenderPipeline,
}

/// A fixed size for the shadow map — generous enough that both example
/// scenes' shadows don't read as visibly blocky, without the per-frame
/// cost a much larger map would add. Not exposed as a `SceneRenderer::new`
/// parameter yet (no caller has needed to tune it) — additive future
/// work if one does.
const SHADOW_MAP_SIZE: u32 = 2048;

impl SceneRenderer {
    pub fn new(device: &Device, surface: &Surface) -> Self {
        let colored_unlit_shader =
            device.create_shader("meridian-unlit-colored", UNLIT_SHADER_WGSL);
        let colored_unlit_pipeline = device.create_render_pipeline(
            &colored_unlit_shader,
            "vs_main",
            "fs_main",
            &colored_vertex_layout(),
            surface,
            true,
        );
        let colored_lit_shader = device.create_shader("meridian-lit-colored", &lit_shader_wgsl());
        let colored_lit_pipeline = device.create_render_pipeline(
            &colored_lit_shader,
            "vs_main",
            "fs_main",
            &colored_vertex_layout(),
            surface,
            true,
        );
        let textured_unlit_shader =
            device.create_shader("meridian-unlit-textured", TEXTURED_SHADER_WGSL);
        let textured_unlit_pipeline = device.create_render_pipeline(
            &textured_unlit_shader,
            "vs_main",
            "fs_main",
            &textured_vertex_layout(),
            surface,
            true,
        );
        let textured_lit_shader =
            device.create_shader("meridian-lit-textured", &lit_textured_shader_wgsl());
        let textured_lit_pipeline = device.create_render_pipeline(
            &textured_lit_shader,
            "vs_main",
            "fs_main",
            &textured_vertex_layout(),
            surface,
            true,
        );

        let shadow_map = device.create_shadow_map_texture(SHADOW_MAP_SIZE);
        let shadow_sampler = device.create_comparison_sampler();
        let shadow_shader = device.create_shader("meridian-shadow-depth", SHADOW_DEPTH_SHADER_WGSL);
        let shadow_colored_pipeline =
            device.create_shadow_pipeline(&shadow_shader, "vs_main", &shadow_from_colored_layout());
        let shadow_textured_pipeline = device.create_shadow_pipeline(
            &shadow_shader,
            "vs_main",
            &shadow_from_textured_layout(),
        );
        let shadow_uniform_buffer = device.create_buffer(64, BufferUsage::Uniform);
        // Just the one `light_view_proj` uniform each (see
        // `SHADOW_DEPTH_SHADER_WGSL`'s single `@binding(0)`) — not
        // `create_shadow_bind_group` (that builds the *lit* shaders'
        // 3-binding uniform+shadow-map+shadow-sampler shape; the depth
        // pass itself never samples the shadow map, only writes it). Two
        // separate bind groups, one per pipeline — see the struct's own
        // doc comment on `shadow_colored_bind_group` for why one shared
        // bind group doesn't validate against both pipelines.
        let shadow_colored_bind_group =
            device.create_uniform_bind_group(&shadow_colored_pipeline, &shadow_uniform_buffer);
        let shadow_textured_bind_group =
            device.create_uniform_bind_group(&shadow_textured_pipeline, &shadow_uniform_buffer);

        // 400 bytes: see `lit_uniform_bytes`'s doc comment for the
        // layout. The unlit pipelines only read the first 64 (view_proj)
        // but share this same buffer — one write serves every pipeline.
        let uniform_buffer = device.create_buffer(400, BufferUsage::Uniform);
        let colored_unlit_bind_group =
            device.create_uniform_bind_group(&colored_unlit_pipeline, &uniform_buffer);
        let colored_lit_bind_group = device.create_shadow_bind_group(
            &colored_lit_pipeline,
            &uniform_buffer,
            &shadow_map,
            &shadow_sampler,
        );
        let sampler = device.create_sampler();

        let renderer = Self {
            colored_unlit_pipeline,
            colored_unlit_bind_group,
            colored_lit_pipeline,
            colored_lit_bind_group,
            textured_unlit_pipeline,
            textured_lit_pipeline,
            uniform_buffer,
            sampler,
            shadow_map,
            shadow_sampler,
            shadow_uniform_buffer,
            shadow_colored_bind_group,
            shadow_textured_bind_group,
            shadow_colored_pipeline,
            shadow_textured_pipeline,
        };

        // One throwaway pass, no draws, purely for its `LoadOp::Clear(1.0)`
        // side effect — guarantees the shadow map always reads back as
        // "no occluder anywhere" (far plane depth) even for a caller
        // that never calls `render_shadow_pass` (e.g. a scene with no
        // directional light), rather than leaving it as newly-allocated
        // GPU memory with undefined content.
        let mut init_commands = device.create_command_buffer();
        {
            let _clear_only = init_commands.begin_shadow_pass(&renderer.shadow_map);
        }
        init_commands.submit();

        renderer
    }

    /// The shared view/lighting uniform buffer — [`crate::bloom::BloomPass`]
    /// binds its own emissive-extraction pipelines against this same
    /// buffer (it needs `view_proj` for its vertex shader), rebuilding a
    /// fresh bind group against it each frame, the same "correct now,
    /// cache later" trade-off already applied to per-texture bind groups
    /// (see the module doc).
    pub(crate) fn uniform_buffer(&self) -> &Buffer {
        &self.uniform_buffer
    }

    /// Writes this view's camera, the shadow-casting light's own
    /// view-projection matrix, ambient and lights into the shared
    /// uniform — call once per frame before [`draw`](Self::draw), after
    /// [`render_shadow_pass`](Self::render_shadow_pass) (which computes
    /// `light_view_proj` and `shadow_caster`). Lights beyond
    /// [`MAX_LIGHTS`] are dropped and logged.
    #[allow(clippy::too_many_arguments)]
    pub fn set_view(
        &self,
        device: &Device,
        camera: &Camera,
        light_view_proj: [[f32; 4]; 4],
        shadow_caster: u32,
        ambient_ground: [f32; 3],
        ambient_sky: [f32; 3],
        lights: &[Light],
    ) {
        if lights.len() > MAX_LIGHTS {
            meridian_foundation::log_warn!(
                "scene has {} lights, dropping {} beyond the cap of {MAX_LIGHTS}",
                lights.len(),
                lights.len() - MAX_LIGHTS
            );
        }
        device.write_buffer(
            &self.uniform_buffer,
            &lit_uniform_bytes(
                camera,
                light_view_proj,
                ambient_ground,
                ambient_sky,
                shadow_caster,
                lights,
            ),
        );
    }

    /// Builds one [`DrawBuffers`] per renderable whose mesh/material both
    /// resolve (a renderable naming a missing handle is silently skipped
    /// — the caller controls registry contents, so a miss here means an
    /// asset hasn't loaded yet, not a bug to panic over). Call this
    /// *before* opening the render pass; pass the result to
    /// [`draw`](Self::draw) once inside it.
    pub fn prepare(
        &self,
        device: &Device,
        renderables: &[Renderable3D],
        meshes: &MeshRegistry,
        materials: &MaterialRegistry,
        textures: &TextureRegistry,
    ) -> Vec<DrawBuffers> {
        renderables
            .iter()
            .filter_map(|renderable| {
                let source = meshes.get(renderable.mesh)?;
                let material = materials.get(renderable.material)?;
                let albedo = material.albedo.and_then(|handle| textures.get(handle));
                Some(bake_draw_buffers(
                    device,
                    self,
                    source,
                    material,
                    &renderable.frame,
                    albedo,
                ))
            })
            .collect()
    }

    /// The shadow-casting depth pass: finds `scene`'s first
    /// [`Light::Directional`] (if any), builds its
    /// [`directional_shadow_camera`], writes that light-space matrix
    /// into the (separate) shadow uniform buffer, bakes `scene`'s draw
    /// buffers (reused afterward by [`draw`](Self::draw) — never
    /// rebuilt), and records a depth-only pass into the internal shadow
    /// map through `commands`. Returns the baked buffers plus
    /// `(light_view_proj, shadow_caster_index)` for the caller to pass
    /// into [`set_view`](Self::set_view) before its own (separate, later)
    /// main color pass — see this module's own doc for why the shadow
    /// pass can't be folded into [`submit_scene3d`] itself (it has to
    /// finish, as its own pass, before the caller's main pass opens on
    /// the same `commands`).
    ///
    /// No [`Light::Directional`] in `scene.lights`: still records an
    /// (empty-of-occluders-mattering) depth pass for consistency, and
    /// returns `shadow_caster = u32::MAX`, which `shade`'s lookup never
    /// matches — every light renders fully lit, the same as before
    /// shadows existed.
    pub fn render_shadow_pass(
        &self,
        device: &Device,
        commands: &mut CommandBuffer<'_>,
        scene: &Scene3D,
        meshes: &MeshRegistry,
        materials: &MaterialRegistry,
        textures: &TextureRegistry,
    ) -> (Vec<DrawBuffers>, [[f32; 4]; 4], u32) {
        let shadow_caster = scene
            .lights
            .iter()
            .position(|l| matches!(l, Light::Directional { .. }))
            .map(|i| i as u32)
            .unwrap_or(u32::MAX);
        let shadow_direction = match scene.lights.get(shadow_caster as usize) {
            Some(Light::Directional { direction, .. }) => direction.transform_vector(Vec3::X),
            _ => Vec3::new(0.0, -1.0, 0.0),
        };
        let shadow_camera = directional_shadow_camera(shadow_direction, Vec3::ZERO);
        let light_view_proj = shadow_camera.view_projection_matrix();
        device.write_buffer(&self.shadow_uniform_buffer, &mat4_to_bytes(light_view_proj));

        let buffers = self.prepare(device, &scene.renderables, meshes, materials, textures);
        {
            let mut pass = commands.begin_shadow_pass(&self.shadow_map);
            for entry in &buffers {
                let (pipeline, bind_group) = match &entry.kind {
                    DrawKind::Colored => (
                        &self.shadow_colored_pipeline,
                        &self.shadow_colored_bind_group,
                    ),
                    DrawKind::Textured { .. } => (
                        &self.shadow_textured_pipeline,
                        &self.shadow_textured_bind_group,
                    ),
                };
                pass.set_pipeline(pipeline);
                pass.set_bind_group(0, bind_group);
                pass.set_vertex_buffer(0, &entry.vertex_buffer);
                pass.set_index_buffer_u16(&entry.index_buffer);
                pass.draw_indexed(0..entry.index_count);
            }
        }

        (buffers, light_view_proj, shadow_caster)
    }

    /// Records one draw call per `buffers` entry against `pass`,
    /// switching pipeline/bind group per entry's baked family and lit
    /// flag — call inside the render pass, after
    /// [`render_shadow_pass`](Self::render_shadow_pass) built `buffers`
    /// and [`set_view`](Self::set_view) wrote this frame's uniform.
    pub fn draw(&self, pass: &mut DriverRenderPass<'_>, buffers: &[DrawBuffers]) {
        for entry in buffers {
            match &entry.kind {
                DrawKind::Colored => {
                    let pipeline = if entry.lit {
                        &self.colored_lit_pipeline
                    } else {
                        &self.colored_unlit_pipeline
                    };
                    let bind_group = if entry.lit {
                        &self.colored_lit_bind_group
                    } else {
                        &self.colored_unlit_bind_group
                    };
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, bind_group);
                }
                DrawKind::Textured { bind_group } => {
                    let pipeline = if entry.lit {
                        &self.textured_lit_pipeline
                    } else {
                        &self.textured_unlit_pipeline
                    };
                    pass.set_pipeline(pipeline);
                    pass.set_bind_group(0, bind_group);
                }
            }
            pass.set_vertex_buffer(0, &entry.vertex_buffer);
            pass.set_index_buffer_u16(&entry.index_buffer);
            pass.draw_indexed(0..entry.index_count);
        }
    }
}

/// The main-pass half of a `Scene3D` view's per-frame path: writes the
/// camera/shadow/lighting uniform and records the draws — call after
/// [`SceneRenderer::render_shadow_pass`] (which supplies `light_view_proj`/
/// `shadow_caster` and the `buffers` to pass through here unchanged) and
/// inside the caller's own already-open main render pass. Doesn't build
/// `buffers` itself (unlike the pre-shadow-mapping version of this
/// function) — see [`SceneRenderer::render_shadow_pass`]'s own doc
/// comment for why baking has to happen once, before either pass opens,
/// not per-pass.
pub fn submit_scene3d(
    device: &Device,
    renderer: &SceneRenderer,
    pass: &mut DriverRenderPass<'_>,
    scene: &Scene3D,
    light_view_proj: [[f32; 4]; 4],
    shadow_caster: u32,
    buffers: &[DrawBuffers],
) {
    renderer.set_view(
        device,
        &scene.camera,
        light_view_proj,
        shadow_caster,
        scene.ambient_ground,
        scene.ambient_sky,
        &scene.lights,
    );
    renderer.draw(pass, buffers);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn triangle() -> MeshSource {
        MeshSource {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
            indices: vec![0, 1, 2],
        }
    }

    /// `m * vec4(p, 1)`, `m` column-major (`m[col][row]`) — the same
    /// convention every matrix in this workspace uses (see
    /// `Frustum::from_view_projection`'s identical row-extraction).
    fn transform_point_via_matrix(m: [[f32; 4]; 4], p: Vec3) -> [f32; 4] {
        let v = [p.x, p.y, p.z, 1.0];
        let mut out = [0.0f32; 4];
        for (row, slot) in out.iter_mut().enumerate() {
            *slot = (0..4).map(|col| m[col][row] * v[col]).sum();
        }
        out
    }

    #[test]
    fn directional_shadow_camera_orders_depth_by_distance_along_light_direction() {
        // Light shines straight down: a point above `scene_center` is
        // closer to the light than one below it.
        let light_direction = Vec3::new(0.0, -1.0, 0.0);
        let camera = directional_shadow_camera(light_direction, Vec3::ZERO);
        let vp = camera.view_projection_matrix();

        let near_clip = transform_point_via_matrix(vp, Vec3::new(0.0, 5.0, 0.0));
        let far_clip = transform_point_via_matrix(vp, Vec3::new(0.0, -5.0, 0.0));
        let near_depth = near_clip[2] / near_clip[3];
        let far_depth = far_clip[2] / far_clip[3];

        assert!(
            near_depth < far_depth,
            "a point closer to the light along its direction must have a smaller \
             depth (near {near_depth}, far {far_depth}) — matches the shadow \
             pipeline's CompareFunction::Less depth test"
        );
    }

    #[test]
    fn directional_shadow_camera_centers_scene_center_in_clip_space() {
        let light_direction = Vec3::new(0.3, -1.0, 0.2);
        let scene_center = Vec3::new(1.0, 2.0, 3.0);
        let camera = directional_shadow_camera(light_direction, scene_center);
        let vp = camera.view_projection_matrix();

        let clip = transform_point_via_matrix(vp, scene_center);
        let ndc_x = clip[0] / clip[3];
        let ndc_y = clip[1] / clip[3];
        assert!(
            ndc_x.abs() < 1e-4,
            "scene_center must map to clip-space x=0, got {ndc_x}"
        );
        assert!(
            ndc_y.abs() < 1e-4,
            "scene_center must map to clip-space y=0, got {ndc_y}"
        );
    }

    #[test]
    fn mesh_registry_rejects_non_triangle_index_counts() {
        let mut registry = MeshRegistry::new();
        let mut source = triangle();
        source.indices = vec![0, 1]; // not a multiple of 3
        assert_eq!(
            registry.register(source),
            Err(MeshRegistryError::NotATriangleList { index_count: 2 })
        );
    }

    #[test]
    fn mesh_registry_rejects_meshes_beyond_u16_vertex_range() {
        let mut registry = MeshRegistry::new();
        let count = u16::MAX as usize + 1;
        let source = MeshSource {
            positions: vec![[0.0, 0.0, 0.0]; count],
            normals: vec![[0.0, 0.0, 1.0]; count],
            uvs: vec![[0.0, 0.0]; count],
            indices: vec![0, 1, 2],
        };
        assert_eq!(
            registry.register(source),
            Err(MeshRegistryError::TooManyVertices { count })
        );
    }

    #[test]
    fn mesh_registry_rejects_uv_count_mismatch() {
        let mut registry = MeshRegistry::new();
        let mut source = triangle();
        source.uvs.pop();
        assert_eq!(
            registry.register(source),
            Err(MeshRegistryError::UvCountMismatch {
                positions: 3,
                uvs: 2
            })
        );
    }

    #[test]
    fn mesh_registry_rejects_normal_count_mismatch() {
        let mut registry = MeshRegistry::new();
        let mut source = triangle();
        source.normals.pop();
        assert_eq!(
            registry.register(source),
            Err(MeshRegistryError::NormalCountMismatch {
                positions: 3,
                normals: 2
            })
        );
    }

    #[test]
    fn mesh_registry_round_trips_a_valid_mesh() {
        let mut registry = MeshRegistry::new();
        let handle = registry.register(triangle()).unwrap();
        assert_eq!(registry.get(handle).unwrap().indices, vec![0, 1, 2]);
        assert!(registry.unregister(handle).is_some());
        assert!(registry.get(handle).is_none());
    }

    #[test]
    fn material_registry_round_trips() {
        let mut registry = MaterialRegistry::new();
        let material = Material {
            base_color_factor: [1.0, 0.0, 0.0, 1.0],
            ..Default::default()
        };
        let handle = registry.register(material);
        assert_eq!(
            registry.get(handle).unwrap().base_color_factor,
            [1.0, 0.0, 0.0, 1.0]
        );
        assert!(registry.unregister(handle).is_some());
        assert!(registry.get(handle).is_none());
    }

    #[test]
    fn mat4_to_bytes_is_column_major_little_endian() {
        let m = [
            [1.0, 2.0, 3.0, 4.0],
            [5.0, 6.0, 7.0, 8.0],
            [9.0, 10.0, 11.0, 12.0],
            [13.0, 14.0, 15.0, 16.0],
        ];
        let bytes = mat4_to_bytes(m);
        assert_eq!(&bytes[0..4], &1.0f32.to_le_bytes());
        assert_eq!(&bytes[16..20], &5.0f32.to_le_bytes());
        assert_eq!(&bytes[60..64], &16.0f32.to_le_bytes());
    }

    #[test]
    fn lit_uniform_bytes_has_the_documented_layout_and_size() {
        let camera = Camera::default();
        let lights = vec![
            Light::Directional {
                direction: Motor3::identity(),
                color: [1.0, 1.0, 1.0],
                intensity: 2.0,
            },
            Light::Point {
                position: Motor3::translation(Vec3::new(1.0, 2.0, 3.0)),
                color: [0.5, 0.25, 0.0],
                intensity: 3.0,
                range: 10.0,
            },
        ];
        let identity = Camera::default().view_projection_matrix();
        let bytes = lit_uniform_bytes(
            &camera,
            identity,
            [0.1, 0.1, 0.1],
            [0.2, 0.2, 0.2],
            0,
            &lights,
        );
        assert_eq!(
            bytes.len(),
            64 + 64 + 16 + 16 + 16 + 16 + 16 + MAX_LIGHTS * 48
        );

        // light_count.x at offset 176 (view_proj[64] + light_view_proj[64]
        // + camera_pos[16] + ambient_ground[16] + ambient_sky[16]) must
        // read back as 2.
        let count = u32::from_le_bytes(bytes[176..180].try_into().unwrap());
        assert_eq!(count, 2);

        // shadow_caster.x at offset 192 (light_count's own 16 bytes
        // later) must read back as the `0` this test passed in.
        let shadow_caster = u32::from_le_bytes(bytes[192..196].try_into().unwrap());
        assert_eq!(shadow_caster, 0);

        // First light's kind field (lights array starts at 208, kind is
        // each 48-byte LightData's 4th f32).
        let lights_start = 64 + 64 + 16 + 16 + 16 + 16 + 16;
        let first_kind = f32::from_le_bytes(
            bytes[lights_start + 12..lights_start + 16]
                .try_into()
                .unwrap(),
        );
        assert_eq!(first_kind, 0.0); // directional
        let second_kind = f32::from_le_bytes(
            bytes[lights_start + 48 + 12..lights_start + 48 + 16]
                .try_into()
                .unwrap(),
        );
        assert_eq!(second_kind, 1.0); // point
    }

    #[test]
    fn lit_uniform_bytes_caps_extra_lights_silently_at_the_byte_level() {
        // The truncation warning is logged by SceneRenderer::set_view;
        // lit_uniform_bytes itself just caps light_count and only writes
        // MAX_LIGHTS slots regardless of how many are passed in.
        let camera = Camera::default();
        let lights: Vec<Light> = (0..(MAX_LIGHTS + 3))
            .map(|_| Light::Directional {
                direction: Motor3::identity(),
                color: [1.0, 1.0, 1.0],
                intensity: 1.0,
            })
            .collect();
        let identity = Camera::default().view_projection_matrix();
        let bytes = lit_uniform_bytes(
            &camera,
            identity,
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            u32::MAX,
            &lights,
        );
        assert_eq!(
            bytes.len(),
            64 + 64 + 16 + 16 + 16 + 16 + 16 + MAX_LIGHTS * 48
        );
        let count = u32::from_le_bytes(bytes[176..180].try_into().unwrap());
        assert_eq!(count as usize, MAX_LIGHTS);
        let shadow_caster = u32::from_le_bytes(bytes[192..196].try_into().unwrap());
        assert_eq!(shadow_caster, u32::MAX);
    }

    /// `TextureRegistry::upload` against a real (or skipped) headless GPU
    /// device — the one piece of this module testable without a window
    /// (`SceneRenderer::new`/`prepare`/`draw` need a real `Surface`,
    /// which needs a window; those are exercised end-to-end by a
    /// windowed example instead, the same convention
    /// `meridian-graphics-driver`'s own module doc states for its
    /// windowed-only paths).
    #[tokio::test]
    async fn texture_registry_round_trips_a_real_upload() {
        let Ok(device) = Device::new().await else {
            eprintln!("skipping: no GPU device available");
            return;
        };
        let mut textures = TextureRegistry::new();
        let image = meridian_asset_core::ImageData {
            width: 1,
            height: 1,
            pixels: vec![255, 255, 255, 255],
        };
        let handle = textures.upload(&device, &image);
        assert!(textures.get(handle).is_some());
        assert!(textures.unregister(handle).is_some());
        assert!(textures.get(handle).is_none());
    }
}

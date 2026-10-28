//! WGSL for looking a surface up in the cascaded shadow atlas.

/// The shadow lookup of the lit shaders (ADR 037, ADR 040). It reads the
/// `u.light_view_proj`, `u.cascade_far` and `u.cascade_texel_depth` uniform
/// fields and the `shadow_map`/`shadow_sampler` bindings, which each lit
/// shader flavor declares at its own binding numbers.
///
/// The atlas is a 2 x 2 grid of tiles, `tile` texels per side. A surface
/// reads the cascade its view distance falls in; in the last tenth of that
/// cascade it also reads the next and blends, so the shadow does not jump at
/// the seam. Each lookup stays one texel inside its tile so the filter never
/// reads a neighbour.
pub(super) const SHADOW_LOOKUP_WGSL: &str = r"
/// The part of a cascade, counted from its far edge, over which it blends
/// into the next.
const CASCADE_BLEND_BAND: f32 = 0.1;

fn cascade_of(view_depth: f32) -> u32 {
    var cascade = 0u;
    if (view_depth > u.cascade_far.x) { cascade = 1u; }
    if (view_depth > u.cascade_far.y) { cascade = 2u; }
    if (view_depth > u.cascade_far.z) { cascade = 3u; }
    return cascade;
}

/// 1 when `world_pos` is lit by the light as `cascade`'s map sees it, 0 in
/// shadow; 1 as well when the point lies outside the map. Compared with
/// `textureSampleCompareLevel`, which needs no uniform control flow, so the
/// blend may skip it.
fn cascade_visibility(world_pos: vec3<f32>, n_dot_l: f32, view_depth: f32, cascade: u32) -> f32 {
    let tile = f32(textureDimensions(shadow_map).x / 2u);
    let clip = u.light_view_proj[cascade] * vec4<f32>(world_pos, 1.0);
    let ndc = clip.xyz / clip.w;
    let uv = vec2<f32>(ndc.x * 0.5 + 0.5, 1.0 - (ndc.y * 0.5 + 0.5));
    // Slope-scaled bias, counted in shadow texels: a surface nearly edge-on
    // to the light (small `n_dot_l`) needs a much bigger depth push than one
    // facing it, or it reads as 'acne' (false self-shadow stripes from the
    // map's own texel quantization). The base term alone still matters for
    // a face exactly perpendicular to the light, where the slope term is 0.
    let bias = max(3.3, 24.6 * (1.0 - n_dot_l)) * u.cascade_texel_depth[cascade];
    let inside = uv.x >= 0.0 && uv.x <= 1.0 && uv.y >= 0.0 && uv.y <= 1.0
        && ndc.z >= 0.0 && ndc.z <= 1.0 && view_depth <= u.cascade_far.w;
    let margin = 1.0 / tile;
    let in_tile = clamp(uv, vec2<f32>(margin), vec2<f32>(1.0 - margin));
    let origin = vec2<f32>(f32(cascade & 1u), f32(cascade >> 1u));
    let atlas_uv = (origin + in_tile) * 0.5;
    let lit = textureSampleCompareLevel(shadow_map, shadow_sampler, atlas_uv, ndc.z - bias);
    return select(1.0, lit, inside);
}

fn compute_shadow_factor(world_pos: vec3<f32>, world_normal: vec3<f32>) -> f32 {
    let n_dot_l = max(dot(normalize(world_normal), normalize(-u.light_direction.xyz)), 0.0);
    let view_depth = (u.view_proj * vec4<f32>(world_pos, 1.0)).w;
    let cascade = cascade_of(view_depth);
    var visibility = cascade_visibility(world_pos, n_dot_l, view_depth, cascade);
    if (cascade < 3u) {
        let edge = u.cascade_far[cascade];
        let blend = clamp((view_depth - edge * (1.0 - CASCADE_BLEND_BAND)) / (edge * CASCADE_BLEND_BAND), 0.0, 1.0);
        if (blend > 0.0) {
            visibility = mix(visibility, cascade_visibility(world_pos, n_dot_l, view_depth, cascade + 1u), blend);
        }
    }
    return visibility;
}
";

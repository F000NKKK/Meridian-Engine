# ADR 037: cascaded shadow maps

## Status

Accepted. Replaces the single fixed shadow volume of the original shadow
mapping (roadmap, "Shadow mapping and hemisphere ambient").

## Context

The directional light's shadow map covered a fixed box of 40 world units
around the origin, whatever the camera saw. A scene larger than the box lost
its shadows at the edge, and a small scene spent most of the map's texels on
empty space. Fitting one map to the view frustum fails the other way: a
frustum that reaches far from the eye leaves the near ground a few texels per
tile. Cascades split the view depth into slices and give each slice its own
fitted map.

## Decisions

**D1 — four cascades, fitted in rendering-domain.** `ShadowCascades::fit` takes
the scene camera and the light direction and returns one light-space
view-projection per cascade, the view distance at which each ends and the
normalised depth of one shadow texel. The slice edges blend logarithmic and
uniform spacing (lambda 0.75) between the camera's near plane and
`min(far, MAX_SHADOW_DISTANCE)`, 100 units. The frustum is read back from the
perspective projection matrix, so `Camera` is unchanged. This is pure math in
the domain with no GPU types, covered by unit tests.

**D2 — each volume is a bounding sphere's box.** The box around the slice's
bounding sphere keeps its size when the camera turns, so shadow edges do not
shimmer. Its origin is snapped to whole shadow texels in light space, so they
do not crawl when the camera moves. The box reaches 50 units toward the light
past the slice so casters outside the view still shadow it, and keeps four
texels of margin at its edge for the shader's filter.

**D3 — fallback.** A camera without a perspective projection (orthographic or
identity) gets `ShadowCascades::fixed`: the old 40-unit box in every cascade,
ending at no distance, with the old bias. Such a view renders as before.

**D4 — one 2 x 2 atlas, one pass.** The adapter keeps one `Depth32Float`
texture of 2 x 2 tiles and the existing `texture_depth_2d` binding, so no
bind group layout or sampler changes. `with_shadow_map_size` now means texels
per tile; the default is 1024, which makes the atlas 2048 along a side like
the single map before. The shadow pass clears the atlas once, then per cascade
sets the viewport to its tile, binds that cascade's matrix uniform and draws
every caster. `RenderPass::set_viewport` is new.

**D5 — the lookup picks the cascade per fragment.** The lit uniform grows from
576 to 800 bytes: four matrices, the cascade far edges and the per-texel depth
sizes replace the single `light_view_proj`. The shader takes a fragment's view
distance from `view_proj`'s `w`, picks the first cascade whose far edge is
beyond it, projects into that tile, clamps one texel inside it and compares.
Fragments past the last cascade are lit. The bias keeps its slope scaling but
is counted in shadow texels (3.3 to 27.9), scaled by the cascade's texel size,
which reproduces the old bias exactly for the fixed volume.

## Consequences

- Shadows reach 100 units from the camera and are sharp near it; a scene
  centred anywhere gets them, not only one around the origin.
- The shadow pass draws every caster four times. There is no per-cascade
  culling yet; it is the first thing to add if shadow casting gets costly.
- Cascade boundaries are a hard switch: no blend band, so a visible seam is
  possible where texel density changes sharply.
- The scene camera, not the view that is drawn, fits the cascades; a second
  view drawn with another camera needs its own `render_shadow_pass`.
- Point-light shadows and soft (PCSS) shadows are still not supported.

# ADR 040: per-cascade caster culling and cascade blending

## Status

Accepted. Refines [ADR 037](037-cascaded-shadow-maps.md).

## Context

ADR 037 named two limits: every shadow caster was drawn into all four
cascades, and the lit shaders switched cascade with a hard edge, so a shadow
changed sharpness along a visible line.

## Decisions

**D1 — each cascade draws only the casters that reach its volume.** The
adapter bakes every instance's world-space box (the mesh's local box through
the instance's frame, `MeshSource::bounds` and `world_space_bounds`), then
tests it against each cascade's orthographic volume
(`ShadowCascades::frusta`, a `Frustum` per cascade). Volumes reach 50 units
toward the light, so a caster outside the view that throws a shadow into it
is kept. An instance whose mesh has no vertices has no box and is kept in every
cascade.

**D2 — the survivors are copied, not drawn by index.** An instanced draw takes
one contiguous range, so each cascade gets its own copy of the surviving
records in one extra vertex buffer, with one `CasterRun` (batch, record range)
per batch and cascade. The copy is made on the CPU each frame next to the
bake; at 120 bytes a record it costs a memcpy per visible caster per cascade.

**D3 — the shader blends over the last tenth of a cascade.** In the band
`[0.9, 1.0] * far_edge` of cascade `c < 3` the lookup reads cascade `c + 1` too
and mixes the two visibilities linearly. The shadow lookup moved to its own
WGSL text (`shadow_lookup_shader`) shared by both lit flavors. It compares
with `textureSampleCompareLevel`, which needs no uniform control flow, so the
second read happens only inside the band.

## Consequences

- A scene with casters spread over a large area draws a fraction of them per
  far cascade; a compact scene draws about the same as before plus the copy.
- The cost of culling is CPU-side and linear in casters times four; a spatial
  structure or GPU culling is future work.
- The last cascade still ends hard at `MAX_SHADOW_DISTANCE`, with no fade.
- The blend band is a constant (`CASCADE_BLEND_BAND`), not a setting.

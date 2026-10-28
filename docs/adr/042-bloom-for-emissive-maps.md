# ADR 042: bloom for emissive-mapped materials

## Status

Accepted. Closes the gap left by [ADR 033](033-normal-occlusion-and-emissive-maps.md)
and [ADR 034](034-hdr-scene-target-and-tone-mapping.md).

## Context

Bloom's bright pass redraws every batch through one pipeline that reads the
plain emissive colour from the instance record. A material with an emissive
map keeps its factor in another slot and zero in the plain one, so a glowing
HUD texture on a helmet did not bloom.

## Decisions

**D1 — a second extraction pipeline samples the map.** `BloomPass` owns a
pipeline for batches with an emissive map: it reads the mesh's position and UV
and the instance's model matrix, plain emissive and map factor, and outputs
`emissive + factor * texture(uv)`. Its shader declares the view uniform at
binding 0, the map at 1 and a sampler at 2, the same slots the sky pass uses,
so the existing bind group helper builds it.

**D2 — the batch names its map.** `PreparedBatch::emissive_map` carries the
`TextureHandle` that `bake_batches` resolved for it. `BloomInput` gains the
`TextureRegistry`, so the pass looks the texture up at draw time and does not
hold GPU objects beyond the frame. The sampler is the renderer's repeating
one, as in the main pass.

**D3 — batches are split between the two pipelines.** Batches without a map
draw through the plain pipeline as before; batches with one draw only through
the sampling pipeline, one bind group each. A batch whose handle no longer
resolves is skipped.

## Consequences

- Emissive-mapped surfaces glow through bloom like plain emissive ones, with
  their texture's own brightness and colour.
- One more draw per mapped batch and one more bind group per mapped batch per
  frame, rebuilt each frame like the other bloom bind groups.
- The map is sampled with the mesh's UV and no mip bias; a map with strong
  minification may shimmer in the bright pass.

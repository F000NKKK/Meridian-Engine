# ADR 046: main-pass culling and bind-group reuse

## Status

Accepted. The main-pass counterpart of [ADR 040](040-per-cascade-casters-and-blending.md),
which culled shadow casters per cascade.

## Context

The main pass drew every instance of every batch, whatever the camera saw, so
a large scene paid for the vertex work of everything behind and beside the
view. Each batch also built a bind group of its own and the draw rebound
pipeline and bind group before every batch, even when the next batch used the
same ones.

## Decisions

**D1 — the main pass draws only what meets the camera's frustum.**
`VisibleBatches::cull` takes the baked batches, the instance records and each
instance's world-space box, and keeps in drawing order the instances whose box
meets the frustum of the camera's view-projection (`Frustum`, the type the
shadow cull uses). The survivors' records are copied side by side into the
frame's instance buffer, each batch's range is rewritten to its copy, and a
batch with nothing visible is dropped. An instance without bounds is kept: an
unknown box must not make an object vanish. Drawing order, and so the
translucent far-to-near sort, is unchanged.

**D2 — shadows still see everything.** A shadow caster outside the view can
shadow what is inside it, so `bake_batches` keeps every record on the CPU and
the shadow pass culls those against its own volumes (ADR 040). The full
instance buffer is no longer uploaded; each pass uploads what it draws.

**D3 — one bind group per pipeline and set of maps.** `BindGroupCache` hands
out the textured bind group for a (`PipelineKey`, `MaterialMaps`) pair, made
the first time and shared (`Arc`) by every batch that binds the same textures.
Batches of different meshes with one material no longer each build a group.

**D4 — the draw skips bindings that are already set.** `PreparedBatch::binds_like`
says whether the next batch uses the pipeline and bind group the last one
bound (flat batches with one pipeline key, textured ones with the same shared
group); the draw then only rebinds the mesh and instance buffers.

## Consequences

- Vertex and fragment work of offscreen instances is gone from the main pass
  and from bloom, which draws the same prepared batches.
- The cull is per instance on the CPU, linear in the scene's instances, and
  copies the survivors' records each frame, as the shadow cull does; it does
  not replace a spatial structure for scenes of many thousand instances.
- The culling test is conservative (a world box against planes): an instance
  just outside a corner may be drawn, one inside is never dropped.
- `PreparedScene::instance_count` counts visible instances, not the scene's.
- `PreparedBatch` and `DrawKind` are `Clone`; the bind group is shared, not
  copied.

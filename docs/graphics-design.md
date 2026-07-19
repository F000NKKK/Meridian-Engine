# Graphics design — `meridian-graphics-driver` + `meridian-graphics-core`

## The split

`graphics-driver` is the hardware abstraction — the engine's equivalent of a
thin Vulkan/DX12/Metal wrapper: devices, command queues, buffers, textures,
shaders, pipelines, synchronization. It has no concept of a scene, a
material, or a camera. In other engines this role is sometimes called an
RHI (Render Hardware Interface); `graphics-driver` *is* that layer under a
different name — not a separate crate to add later. Concrete backends
(`vulkan-driver`, `dx12-driver`, `metal-driver`) are expected to become
their own crates implementing `graphics-driver`'s abstraction, without
`graphics-core` ever changing — see
[ADR 005](adr/005-driver-core-separation.md). No backend crate exists yet;
adding one is out of scope until there's a concrete backend to build (see
[roadmap.md](roadmap.md)).

`graphics-core` is everything that *does* know about scenes:

```text
Render Graph        automatic pass ordering + resource dependency tracking  -- real
Scene Extraction     pulling renderable state out of the ECS each frame
Visibility / Culling -- real (frustum vs AABB)
Lighting
Materials
Camera                -- real (Motor3 -> view/projection matrix bridge)
Animation
Post Processing
```

`Camera`, frustum culling, and render graph pass ordering are real and
tested (`cargo test -p meridian-graphics-core`; human-readable version via
`./build.sh run graphics_validation`) — see below for how each works.
Scene extraction, lighting, materials-as-shading-inputs, animation and post
processing are still scaffolds, blocked on `graphics-driver`/`wgpu` (see
[roadmap.md](roadmap.md)) since they need an actual GPU resource to shade.

## `Motor3` to view/projection matrix

GPU pipelines take a classical column-major 4x4 matrix, not a `Motor3` —
that's a hard constraint of how vertex shaders work, not a design choice.
So `Camera` is the one place in `graphics-core` that converts:

- `Motor3::to_mat4` (in `gac-core` — pure bridging math, not a graphics
  concept, so it lives with the algebra it converts; see
  [gac-design.md](gac-design.md)) turns any `Motor3` into a homogeneous
  matrix by evaluating `transform_point` at the origin and each basis
  vector — exact, not a numerical approximation, since `transform_point` is
  already an affine map.
- `graphics-core`'s local camera convention is forward `+X`, up `+Y`, right
  `+Z` — the same listener-local convention `audio-core` already commits
  to, reused here so a character's camera and ears agree on "forward"
  without either subsystem inventing its own axis convention. `gac-core`
  itself has no "forward" concept (see [gac-design.md](gac-design.md)), so
  this choice, and the fixed remap matrix that applies it
  (`LOCAL_TO_VIEW_REMAP`), live in `graphics-core`.
- `Projection::perspective`/`orthographic` (also in `gac-core`, since
  `Projection` is a documented core type — camera/projective mappings) build
  the view-to-clip matrix: column-major, column-vector convention, depth
  range `[0, 1]` (wgpu/DX12/Metal convention, not classic OpenGL's
  `[-1, 1]`).
- `Camera::view_matrix`/`view_projection_matrix` compose the above: the
  camera's world `Motor3` inverted, remapped into view space, then
  multiplied by the projection.

`Motor3` stays the single source of truth for spatial state everywhere
else in the engine (ECS `Transform`, physics `RigidBody`, audio
`Listener`/`Emitter`); the matrix is computed on demand only where a GPU
API actually needs one, never stored as parallel state.

## Frustum culling

`Frustum::from_view_projection` extracts six half-space planes from a
view-projection matrix via the Gribb/Hartmann method (each plane's
coefficients are a linear combination of the matrix's rows); `gac-core`'s
`Aabb` (shared with `physics-core`'s broad phase — a bounding box has no
graphics- or physics-specific meaning, so it lives once in `gac-core`
rather than being redefined per subsystem, see docs/gac-design.md) plus
`Frustum::intersects_aabb` is the standard conservative box/frustum test
(check the AABB's corner furthest along each plane's normal). This is
generic once the planes exist — extracting them is what ties culling to a
specific `Camera`.

## Render graph over hand-ordered passes

Instead of a hardcoded `Shadow → Geometry → Lighting → Postprocess`
sequence, passes declare their resource reads/writes
(`RenderPass::reading`/`RenderPass::writing`, identified by an opaque
per-graph `GraphResourceId` — transient and frame-scoped, not
`resource-core`'s persistent `ResourceId`) and
`RenderGraph::execution_order` derives execution order automatically: a
producer always runs before every pass that reads what it wrote, using the
same dependency-declared-not-hand-sequenced Kahn's-algorithm idea as
`task-core`'s `JobGraph`, applied to resource conflicts instead of explicit
job dependencies. Two passes writing the same resource, or a read/write
cycle, are rejected rather than silently guessed at. This is what allows
adding a pass without manually re-threading every pass after it. Resource
*lifetime* (when a transient render target is actually allocated/freed)
isn't derived yet — ordering is the real, tested piece; lifetime tracking
is future work once there's a concrete GPU backend to allocate against.

## GPU-driven rendering

The long-term direction is minimizing CPU-side per-object work: the CPU
prepares data, the GPU (via indirect draw, compute-based culling, and
instancing) decides what actually gets drawn. This is why `graphics-core`
depends on `meridian-compute-runtime` — GPU culling and compute-based
visibility are compute workloads, not rendering-specific ones (see
[dependency-rules.md](dependency-rules.md) rule 5).

## What this crate does not own

Asset bytes and decoding belong to `asset-core`; `graphics-core` consumes
already-decoded CPU-side representations and turns them into GPU resources
via `graphics-driver`. It does not implement image/mesh decoders itself.

`TextureHandle`, `MeshHandle`, `BufferHandle` and `ShaderHandle` are typed
resource identities from `meridian-resource-core`, not ad hoc handles
defined in this crate — see
[ADR 006](adr/006-resource-core-separation.md).

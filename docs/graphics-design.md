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
tested (`cargo test -p meridian-graphics-core`) — see below for how each works.
`graphics-driver` itself is real now too: a headless `wgpu` `Device`
(`Buffer`/`Texture`/`Shader`/compute `Pipeline`/`CommandBuffer`, no
window/surface — see [roadmap.md](roadmap.md)'s `wgpu` entry). Scene
extraction, lighting, materials-as-shading-inputs, animation and post
processing are still scaffolds: they need a window/swapchain surface to
present to and a mesh/material vocabulary, neither of which exists yet —
not blocked on `graphics-driver` having a GPU device anymore, just on
those two separate follow-ups.

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

`Frustum::from_view_projection` extracts six `gac-core::Plane` half-spaces
from a view-projection matrix via the Gribb/Hartmann method (each plane's
coefficients are a linear combination of the matrix's rows), then wraps
them in a `gac-core::ConvexVolume`. Extracting exactly six planes from a
`Projection` is what's camera-specific and stays here; the volume/shape
math underneath it is generic and lives once in `gac-core` (see
docs/gac-design.md), so `Frustum::intersects` works against *any*
`gac-core::Shape` — `Aabb`, `Sphere`, `Obb`, `Cone`, or a future one — not
just an AABB. `Frustum` itself owns nothing but the plane extraction; if a
non-camera convex volume is ever needed (a spotlight's volume, a trigger
region), it's `gac-core::ConvexVolume` directly, no `graphics-core`
involvement required.

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

## Scene vocabulary: 2D scenes, 3D scenes, and UI

*Design accepted; implementation staged — see "Implementation order"
below and roadmap.md for current status.*

### One spatial model, two scene kinds

The workspace's core commitment is **one spatial model** (`Motor3`,
ADR 001) — so a 2D scene is *not* a parallel 2D math stack (no separate
`Transform2D`/`Rotor2` universe to keep in sync). Both scene kinds use
`Motor3` frames; what differs is the camera and the conventions layered
on top:

- **`Scene3D`** — the world: perspective (or orthographic) [`Camera`],
  entities at arbitrary `Motor3` frames, frustum culling, lights,
  depth-tested.
- **`Scene2D`** — a plane: entities constrained to the local `z = 0`
  plane of the scene (position = `x`/`y` of the frame's translation,
  rotation = rotation about the plane normal, the remaining `Motor3`
  degrees of freedom simply unused), rendered with an orthographic
  camera at a fixed distance. Painter's-order sorting by an explicit
  integer `layer` (plus `y`/submission order within a layer), no
  perspective, no lights in the first cut. Units are whatever the
  orthographic camera says they are: world units for 2D gameplay,
  logical pixels for screen-space UI — the camera, not the entity,
  decides.

A frame renders an ordered list of **views** — each view is one scene +
one camera + one target. The common compositions: `[Scene3D]` (a pure 3D
game), `[Scene2D]` (a pure 2D game), `[Scene3D, Scene2D-as-UI-overlay]`
(the usual case). Views later in the list draw over earlier ones.

### UI is a role, not a third scene kind

UI does not get its own rendering vocabulary. A UI is either:

- **Screen-space UI (2D)** — a `Scene2D` whose orthographic camera is
  derived from the surface size (units = logical pixels, origin
  top-left, `y` down — the convention every windowing/UI system shares),
  composed as the last view of the frame, undepthed, drawn over the 3D
  world. This is the HUD/menu case.
- **World-space UI (3D)** — renderables *inside* the `Scene3D` whose
  content happens to be interface (a health bar above a unit, a control
  panel in a cockpit/VR): a quad at a `Motor3` frame like any other
  mesh, optionally flagged `billboard` (re-oriented to face the camera
  at extraction time) and/or `unlit`/`always_on_top` in its material.
  It's culled, depth-tested and lit (or not, per material) by the same
  3D pipeline — no special UI pass.

What UI adds over plain scenes is *content*, not rendering: widgets,
layout, text, input routing. All of that is deliberately out of
`graphics-core` — a future `ui-core` consumes this vocabulary (it emits
`Scene2D`/`Scene3D` renderables) the same way `physics-core` consumes
`gac-core` shapes. Text/font rendering (glyph atlases, shaping) is its
own future decision with its own ADR; nothing in this vocabulary blocks
on it — a text run will extract to textured quads like everything else.

### The types

All CPU-side, plain data, in `graphics-core`:

- **`Mesh`** — a `MeshHandle` identity (resource-core) plus CPU-side
  layout info (vertex count, index count, bounds `Aabb` for culling).
  The actual GPU buffers live behind the submission bridge; `Mesh`
  never owns driver objects.
- **`Material`** (extending the existing stub) — shading inputs only:
  base color factor, optional `TextureHandle` albedo, `unlit: bool`,
  `always_on_top: bool` (world-space UI), blend mode
  (`Opaque`/`AlphaBlend`). Not a manager, not a shader: the submission
  bridge maps materials onto pipelines.
- **`Light`** — `Directional { direction, color, intensity }` and
  `Point { position, color, intensity, range }`. First lighting model is
  Blinn-Phong forward; PBR is a material/shader upgrade later, not a
  vocabulary change.
- **`Renderable3D`** — `{ mesh, material, frame: Motor3, billboard:
  bool }`; **`Sprite`** (the 2D renderable) — `{ texture, source_rect,
  size, tint, frame: Motor3, layer: i32 }`.
- **`Scene3D`** — `{ camera: Camera, renderables: Vec<Renderable3D>,
  lights: Vec<Light> }`; **`Scene2D`** — `{ camera: Camera2D, sprites:
  Vec<Sprite> }` where `Camera2D` is a thin orthographic wrapper
  (`from_surface_size` for pixel-space UI, `from_world_extent` for 2D
  gameplay).
- **`FrameScene`** — the per-frame extraction output: an ordered
  `Vec<View>` where `View` is `Three(Scene3D)` or `Two(Scene2D)`.
  Frame-scoped plain data, rebuilt every frame (extraction output, ADR
  004's data-oriented stance) — *not* a retained scene graph.

### Handles are the identity mechanism — 100% compliance target

Everything a renderable references crosses the vocabulary by **typed
`resource-core` handle** (`MeshHandle`, `TextureHandle`, and a new
`MaterialHandle`), never by value, index, or driver object — ADR 002 is
a hard requirement of this design, not a suggestion. The submission
bridge owns the handle→GPU-resource resolution, backed by
`memory-core`'s generational pools (a stale handle after an asset
reload resolves to "not resident", never to someone else's mesh — the
exact bug class generational handles exist to kill). Existing spots
where the workspace under-complies (examples holding raw
`graphics-driver` buffers, `Material` embedding its texture reference
directly) are acceptable *below* this vocabulary or *before* it lands,
but the extract→cull→submit path itself ships handle-clean from day
one, and the examples converge onto it in step 2 — driving the whole
tree toward full ADR 002/003/006 compliance rather than accreting more
exceptions.

### Extraction and ownership

Persistent state stays where it already lives: the application/ECS owns
entities (`Transform` + render components), `engine-core` orchestrates.
Per frame: **extract** (walk the ECS `World`, build `FrameScene` —
`graphics-core` provides the component types and extraction functions;
it may depend on `ecs-core` per the existing graph, and `ecs-core`
learns nothing about graphics, per rule 6) → **cull** (frustum vs.
`Mesh` bounds for 3D; camera-rect vs. sprite bounds for 2D) →
**submit** (the bridge turns the culled `FrameScene` into render-graph
passes and `graphics-driver` draw calls). Extraction is pure
CPU-testable data flow; the bridge is the only place that touches the
driver.

### Implementation order

1. **Done.** Vocabulary + `FrameScene` + extraction + culling — CPU-only,
   fully tested without a GPU (`extract_scene3d`/`cull_scene3d` in
   `scene.rs`).
2. **Done.** Submission bridge over `graphics-driver` (`submission.rs`):
   `MeshRegistry`/`MaterialRegistry`/`TextureRegistry` (handle-addressed,
   ADR 002-compliant), `SceneRenderer::prepare`/`draw`/`submit_scene3d`
   turning a culled `Scene3D` into real draw calls. Texture upload from
   `asset-core::ImageData` is real: `gpu-driver` gained
   `Device::write_texture`/`create_sampler`/`create_texture_bind_group`,
   `graphics-driver` gained the matching `create_texture_2d`/
   `create_textured_bind_group` passthroughs, and `submission.rs` picks
   between two unlit pipelines per renderable — `TEXTURED_SHADER_WGSL`
   (samples `Material::albedo` when it resolves in the `TextureRegistry`)
   or `UNLIT_SHADER_WGSL` (flat `base_color_factor` otherwise). Still
   disclosed, not hidden, as scoped down: no *lighting* yet (both
   pipelines are unlit — that's step 3), and no GPU instancing (no
   per-instance uniform mechanism exists, so world-space vertices and
   textured-draw bind groups are baked/rebuilt fresh every frame — see
   the module doc's "no per-instance uniform" note and the
   GPU-driven-rendering section below). One windowed example
   (`music_sphere`) has converged onto the bridge — procedurally
   generated checkerboard floor (`TextureRegistry::upload` on a
   generated `ImageData`, no file needed), a lit colored cube, and the
   music-emitting sphere as an unlit+emissive material with `BloomPass`
   applied, replacing that example's hand-rolled pipelines/buffers
   entirely. The remaining windowed examples (`spinning_cube`, the
   soft-body ones) haven't converged yet — real PNG/JPEG decoders (glTF
   once real meshes ship — the ADR 013 when-a-concrete-asset-needs-it
   pattern) and the GPU-compute audit under real load are also still
   open, unblocked by the texture path landing but not yet done.
3. **Done, core lighting; material handling partially open.**
   `submission.rs` gained Blinn-Phong forward lighting: `Scene3D` now
   carries `ambient: [f32; 3]`, and every renderable's family (colored/
   textured) crosses with lit/unlit into four main pipelines —
   `lit_shader_wgsl()`/`lit_textured_shader_wgsl()` shade against
   `Scene3D::lights` (directional + point, capped at `MAX_LIGHTS = 4`,
   extra lights dropped and logged) and `Scene3D::ambient`; the existing
   `UNLIT_SHADER_WGSL`/`TEXTURED_SHADER_WGSL` stay for `Material::unlit`.
   `Material::emissive` bakes into every vertex regardless of family and
   feeds bloom's bright-pass mask (see next). Meshes now carry per-vertex
   normals (`MeshSource::normals`, transformed by `Motor3::transform_vector`
   — rotation only, matching a normal's transform rule). **Bloom is
   real** too, one layer up from the design's original "Post Processing"
   scaffold entry: `crate::bloom::BloomPass` redraws each frame's already-
   baked `DrawBuffers` through an emissive-extraction pipeline into an
   offscreen texture, separably Gaussian-blurs it (two full-screen-
   triangle passes, horizontal then vertical), and additively composites
   the result onto the already-rendered view via the new
   `CommandBuffer::begin_render_pass_loaded` (preserves existing content
   instead of clearing) — deliberately non-HDR (`Rgba8UnormSrgb`
   throughout; `Rgba16Float` + exposure is real future work if a scene
   ever needs it). Still open: real per-material roughness/specular
   control (shininess is fixed at 32 in the shader), shadow mapping, and
   blend-mode/`always_on_top` handling (both fields exist on `Material`
   but the submission bridge doesn't act on either yet — every draw is
   still opaque, depth-tested).
4. `Runtime::tick` integration (step 9 closes).
5. `ui-core` (widgets/layout/input) and text rendering — separate,
   ADR-gated.

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

# Graphics design — `meridian-rendering-wgpu` + `meridian-rendering-domain`

## The split

`rendering-wgpu` is the hardware abstraction — the engine's equivalent of a
thin Vulkan/DX12/Metal wrapper: devices, command queues, buffers, textures,
shaders, pipelines, synchronization. It has no concept of a scene, a
material, or a camera. In other engines this role is sometimes called an
RHI (Render Hardware Interface); `rendering-wgpu` *is* that layer under a
different name — not a separate crate to add later. Concrete backends
(`vulkan-driver`, `dx12-driver`, `metal-driver`) are expected to become
their own crates implementing `rendering-wgpu`'s abstraction, without
`rendering-domain` ever changing. The dependency direction is inverted
([ADR 019](adr/019-ports-in-domain-drivers-as-adapters.md), [ADR 024](adr/024-wave-5-ports-and-adapters.md)):
`rendering-app` owns the ports (mesh/texture stores, frame recording)
and `rendering-wgpu` implements them; `rendering-domain` has no edge to it. No backend crate exists yet;
adding one is out of scope until there's a concrete backend to build (see
[roadmap.md](roadmap.md)).

`rendering-domain` is everything that *does* know about scenes:

```text
Render Graph        automatic pass ordering + resource dependency tracking  -- real
Scene Extraction     pulling renderable state out of the ECS each frame     -- real
Visibility / Culling -- real (frustum vs AABB)
Lighting             -- real (Blinn-Phong forward, directional + point)
Materials            -- real (albedo/base color, unlit, emissive, blend mode field)
Camera                -- real (Motor3 -> view/projection matrix bridge)
Animation
Post Processing      -- real (bloom); HDR/tone-mapping still open
```

`Camera`, frustum culling, render graph pass ordering, scene extraction,
lighting and the submission bridge are all real and tested (`cargo test
-p meridian-rendering-domain`) — see "Implementation order" below for the
step-by-step history and exactly what's still open (HDR). `rendering-wgpu` is real too, both headless (`Device::new`) and
windowed (`Device::new_windowed`, a real swapchain `Surface` —
`Buffer`/`Texture`/`Shader`/compute `Pipeline`/`CommandBuffer` either
way). `Animation` is the one row above with no work started yet — no
vocabulary, no scaffold.

## `Motor3` to view/projection matrix

GPU pipelines take a classical column-major 4x4 matrix, not a `Motor3` —
that's a hard constraint of how vertex shaders work, not a design choice.
So `Camera` is the one place in `rendering-domain` that converts:

- `Motor3::to_mat4` (in `gac` — pure bridging math, not a graphics
  concept, so it lives with the algebra it converts; see
  [gac-design.md](gac-design.md)) turns any `Motor3` into a homogeneous
  matrix by evaluating `transform_point` at the origin and each basis
  vector — exact, not a numerical approximation, since `transform_point` is
  already an affine map.
- `rendering-domain`'s local camera convention is forward `+X`, up `+Y`, right
  `+Z` — the same listener-local convention `audio-domain` already commits
  to, reused here so a character's camera and ears agree on "forward"
  without either subsystem inventing its own axis convention. `gac`
  itself has no "forward" concept (see [gac-design.md](gac-design.md)), so
  this choice, and the fixed remap matrix that applies it
  (`LOCAL_TO_VIEW_REMAP`), live in `rendering-domain`.
- `Projection::perspective`/`orthographic` (also in `gac`, since
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

`Frustum::from_view_projection` extracts six `gac::Plane` half-spaces
from a view-projection matrix via the Gribb/Hartmann method (each plane's
coefficients are a linear combination of the matrix's rows), then wraps
them in a `gac::ConvexVolume`. Extracting exactly six planes from a
`Projection` is what's camera-specific and stays here; the volume/shape
math underneath it is generic and lives once in `gac` (see
docs/gac-design.md), so `Frustum::intersects` works against *any*
`gac::Shape` — `Aabb`, `Sphere`, `Obb`, `Cone`, or a future one — not
just an AABB. `Frustum` itself owns nothing but the plane extraction; if a
non-camera convex volume is ever needed (a spotlight's volume, a trigger
region), it's `gac::ConvexVolume` directly, no `rendering-domain`
involvement required.

## Render graph over hand-ordered passes

Instead of a hardcoded `Shadow → Geometry → Lighting → Postprocess`
sequence, passes declare their resource reads/writes
(`RenderPass::reading`/`RenderPass::writing`, identified by an opaque
per-graph `GraphResourceId` — transient and frame-scoped, not
`resource`'s persistent `ResourceId`) and
`RenderGraph::execution_order` derives execution order automatically: a
producer always runs before every pass that reads what it wrote, using the
same dependency-declared-not-hand-sequenced Kahn's-algorithm idea as
`runtime-threads`'s `JobGraph`, applied to resource conflicts instead of explicit
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

- **`Scene3d`** — the world: perspective (or orthographic) [`Camera`],
  entities at arbitrary `Motor3` frames, frustum culling, lights,
  depth-tested.
- **`Scene2d`** — a plane: entities constrained to the local `z = 0`
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
one camera + one target. The common compositions: `[Scene3d]` (a pure 3D
game), `[Scene2d]` (a pure 2D game), `[Scene3d, Scene2d-as-UI-overlay]`
(the usual case). Views later in the list draw over earlier ones.

### UI is a role, not a third scene kind

UI does not get its own rendering vocabulary. A UI is either:

- **Screen-space UI (2D)** — a `Scene2d` whose orthographic camera is
  derived from the surface size (units = logical pixels, origin
  top-left, `y` down — the convention every windowing/UI system shares),
  composed as the last view of the frame, undepthed, drawn over the 3D
  world. This is the HUD/menu case.
- **World-space UI (3D)** — renderables *inside* the `Scene3d` whose
  content happens to be interface (a health bar above a unit, a control
  panel in a cockpit/VR): a quad at a `Motor3` frame like any other
  mesh, optionally flagged `billboard` (re-oriented to face the camera
  at extraction time) and/or `unlit`/`always_on_top` in its material.
  It's culled, depth-tested and lit (or not, per material) by the same
  3D pipeline — no special UI pass.

What UI adds over plain scenes is *content*, not rendering: widgets,
layout, text, input routing. All of that is deliberately out of
`rendering-domain` — `ui-domain` ([ADR 018](adr/018-ui-architecture.md)) consumes this vocabulary (it emits
`Scene2d`/`Scene3d` renderables) the same way `physics-domain` consumes
`gac` shapes. Text/font rendering (glyph atlases, shaping) is its
own future decision with its own ADR; nothing in this vocabulary blocks
on it — a text run will extract to textured quads like everything else.

### The types

All CPU-side, plain data, in `rendering-domain`:

- **`Mesh`** — a `MeshHandle` identity (resource) plus CPU-side
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
- **`Renderable3d`** — `{ mesh, material, frame: Motor3, billboard:
  bool }`; **`Sprite`** (the 2D renderable) — `{ texture, source_rect,
  size, tint, frame: Motor3, layer: i32 }`.
- **`Scene3d`** — `{ camera: Camera, renderables: Vec<Renderable3d>,
  lights: Vec<Light> }`; **`Scene2d`** — `{ camera: Camera2d, sprites:
  Vec<Sprite> }` where `Camera2d` is a thin orthographic wrapper
  (`from_surface_size` for pixel-space UI, `from_world_extent` for 2D
  gameplay).
- **`FrameScene`** — the per-frame extraction output: an ordered
  `Vec<View>` where `View` is `Three(Scene3d)` or `Two(Scene2d)`.
  Frame-scoped plain data, rebuilt every frame (extraction output, ADR
  004's data-oriented stance) — *not* a retained scene graph.

### Handles are the identity mechanism — 100% compliance target

Everything a renderable references crosses the vocabulary by **typed
`resource` handle** (`MeshHandle`, `TextureHandle`, and a new
`MaterialHandle`), never by value, index, or driver object — ADR 002 is
a hard requirement of this design, not a suggestion. The submission
bridge owns the handle→GPU-resource resolution, backed by
`memory`'s generational pools (a stale handle after an asset
reload resolves to "not resident", never to someone else's mesh — the
exact bug class generational handles exist to kill). Existing spots
where the workspace under-complies (examples holding raw
`rendering-wgpu` buffers, `Material` embedding its texture reference
directly) are acceptable *below* this vocabulary or *before* it lands,
but the extract→cull→submit path itself ships handle-clean from day
one, and the examples converge onto it in step 2 — driving the whole
tree toward full ADR 002/003/006 compliance rather than accreting more
exceptions.

### Extraction and ownership

Persistent state stays where it already lives: the application/ECS owns
entities (`Transform` + render components), `runtime-app` orchestrates.
Per frame: **extract** (walk the ECS `World`, build `FrameScene` —
`rendering-domain` provides the component types and extraction functions;
it may depend on `ecs` per the existing graph, and `ecs`
learns nothing about graphics, per rule 6) → **cull** (frustum vs.
`Mesh` bounds for 3D; camera-rect vs. sprite bounds for 2D) →
**submit** (the bridge turns the culled `FrameScene` into render-graph
passes and `rendering-wgpu` draw calls). Extraction is pure
CPU-testable data flow; the bridge is the only place that touches the
driver.

### Implementation order

1. **Done.** Vocabulary + `FrameScene` + extraction + culling — CPU-only,
   fully tested without a GPU (`extract_scene3d`/`cull_scene3d` in
   `scene.rs`).
2. **Done.** Submission bridge over `rendering-wgpu` (`submission/`):
   `MeshRegistry`/`MaterialRegistry`/`TextureRegistry` (handle-addressed,
   ADR 002-compliant), `SceneRenderer::prepare`/`draw`/`submit_scene3d`
   turning a culled `Scene3d` into real draw calls. Texture upload from
   `asset-domain::ImageAsset` is real: `gpu-wgpu` gained
   `Device::write_texture`/`create_sampler`/`create_texture_bind_group`,
   `rendering-wgpu` gained the matching `create_texture_2d`/
   `create_textured_bind_group` passthroughs, and `submission/` picks
   between two unlit pipelines per renderable — `TEXTURED_SHADER_WGSL`
   (samples `Material::albedo` when it resolves in the `TextureRegistry`)
   or `UNLIT_SHADER_WGSL` (flat `base_color_factor` otherwise). Still
   disclosed, not hidden, as scoped down: no *lighting* yet (both
   pipelines are unlit — that's step 3), and no GPU instancing (no
   per-instance uniform mechanism exists, so world-space vertices and
   textured-draw bind groups are baked/rebuilt fresh every frame — see
   the module doc's "no per-instance uniform" note and the
   GPU-driven-rendering section below). Two windowed examples
   (`magic_figures`, `physic_figures`) have converged onto the bridge —
   a real PNG-textured checkerboard floor (`AnyImageDecoder`/
   `TextureRegistry::upload` on a decoded file, not a generated
   pattern), real BMP/PNG textures per shape, and (in `magic_figures`)
   each orbiting shape as an unlit+emissive material with `BloomPass`
   applied, replacing the old hand-rolled pipelines/buffers entirely.
   `magic_figures`/`physic_figures` are the only two windowed examples in
   the workspace now (`spinning_cube` and the standalone soft-body
   examples were folded into them during a later rename/consolidation —
   see git history if you're chasing an old reference to either name),
   so there's no other windowed example left to converge. glTF decoding
   (once real meshes ship — the ADR 013 when-a-concrete-asset-needs-it
   pattern) and the GPU-compute audit under real load are still open,
   unblocked by the texture path landing but not yet done.
3. **Done, core lighting; material handling partially open.**
   `submission/` gained Blinn-Phong forward lighting: `Scene3d` now
   carries `ambient: [f32; 3]`, and every renderable's family (colored/
   textured) crosses with lit/unlit into four main pipelines —
   `lit_shader_wgsl()`/`lit_textured_shader_wgsl()` shade against
   `Scene3d::lights` (directional + point, capped at `MAX_LIGHTS = 4`,
   extra lights dropped and logged) and `Scene3d::ambient`; the existing
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
   ever needs it). Highlights are per material:
   `Material::shininess` (the Blinn-Phong exponent, default 32) and
   `Material::specular` (strength, default 0.3) are baked into each vertex
   as a `gloss` pair and read by the lit shaders; a PBR upgrade replaces
   them with roughness/metallic.
   **Material states are real:** `DrawState::of(material)` (domain) maps
   `blend` and `always_on_top` to `Opaque`, `Transparent` (alpha blended,
   depth tested but not written), `OnTopOpaque` and `OnTopTransparent`
   (depth ignored). `order_draws` issues them in that order, translucent
   draws farthest first by distance from the camera to the renderable's
   origin (per renderable, not per triangle: intersecting translucent
   meshes do not sort correctly), on-top draws after everything else.
   `MainPipelines` holds one pipeline per family × lighting × state;
   bloom still redraws every draw, so translucent and on-top emissive
   surfaces bloom as if solid.
4. `Runtime::tick` integration — **not** blocked on vocabulary anymore
   (steps 1-3 above cover it); blocked on an actual architecture
   decision instead, since presenting a frame needs a windowed
   `Device`/`Surface` (driver state) and `runtime-app` deliberately
   never depends on `rendering-wgpu`. `magic_figures`/`physic_figures`
   compose `rendering-domain::SceneRenderer` directly against their own
   `Device`/`Surface` inside their own `winit` event loop today, rather
   than through `Runtime::tick` — see docs/roadmap.md's
   `Runtime`-adoption entry for the current state of that decision.
5. **Done.** The 2D sprite path (`sprites/`): `batch_scene2d` culls a
   `Scene2d` to its camera, orders sprites by `layer` (stable) and merges
   consecutive same-texture sprites into `SpriteBatch`es (CPU only, tested
   without a GPU); `SpriteRenderer::prepare`/`draw` upload and record them
   with a no-depth, premultiplied-alpha pipeline
   (`Device::create_premultiplied_alpha_pipeline`). Contract: camera units
   in, `Sprite::frame` is the quad centre (rotation about the plane normal
   turns it), `source_rect` is normalized, `tint` is straight RGBA and the
   shader premultiplies. Plain colored rectangles use a 1x1 white texture
   (`TextureRegistry::upload_solid`) and carry the color in `tint`.
   **Raw path:** `SpriteBatch`/`SpriteVertex` are public, so custom drawing
   builds triangle lists in camera units and calls `prepare` directly,
   never going through `Sprite`. **Clipping:** `ClipSpan { layers, rect }`
   clips a contiguous range of layers (a scroll viewer's content) to a
   `ClipRect`; spans nest by intersection; `batch_scene2d_clipped` tags
   batches and splits them at clip changes, `SpriteRenderer::draw` sets a
   scissor per run (`ClipRect::to_scissor` rounds outward from camera units
   to pixels, so `SpriteView` carries the target size). Clipping is
   rectangular and axis-aligned in camera units; no rounded or rotated
   clips. `prepare` creates one bind group per distinct texture
   per scene (`PreparedSprites::bind_groups`), so clipped runs of one texture
   share it; the bind group still lives one frame, because it holds that
   frame's camera uniform. Batching 10 000 sprites takes about 0.3 ms
   (`benches/sprite_batching.rs`, after the hand-expanded `Motor3` point
   transform), so it no longer dominates. Glyph atlas packing lives in
   `text-domain` (`GlyphAtlas`); a general RGBA sprite atlas waits for a
   consumer.
6. **Done.** Composing `[Scene3d, Scene2d]`: `FrameRequest` carries
   `overlays: &[Overlay2d]` (a `Scene2d` plus its `ClipSpan`s, a
   `rendering-domain` type); `RenderSession::render_with_overlays` hands them to
   the `FrameRecorder`, and `GpuFrameRecorder` draws the 3D scene, applies
   bloom, then draws each overlay unblurred and without depth over the result
   (`OverlayPass`), in slice order. `meridian_sdk::compose_frame` and
   `RenderStage::with_overlays` are the application entry points. Verified
   headless under Xvfb + lavapipe with `magic_figures` (a translucent clipped
   HUD over the bloomed scene, no `wgpu` validation errors).
7. `ui-domain` (widgets/layout/input) and text rendering — separate,
   ADR-gated.

## GPU-driven rendering

The long-term direction is minimizing CPU-side per-object work: the CPU
prepares data, the GPU (via indirect draw, compute-based culling, and
instancing) decides what actually gets drawn. This is why `rendering-domain`
depends on `meridian-compute-app` — GPU culling and compute-based
visibility are compute workloads, not rendering-specific ones (see
[dependency-rules.md](dependency-rules.md) rule 5).

## What this crate does not own

Asset bytes and decoding belong to `asset-domain`; `rendering-domain` consumes
already-decoded CPU-side representations and turns them into GPU resources
via `rendering-wgpu`. It does not implement image/mesh decoders itself.

`TextureHandle`, `MeshHandle`, `BufferHandle` and `ShaderHandle` are typed
resource identities from `meridian-resource`, not ad hoc handles
defined in this crate — see
[ADR 006](adr/006-resource-core-separation.md).

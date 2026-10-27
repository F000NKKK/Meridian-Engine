# Segment: rendering (`team/rendering`)

Ownership note and working backlog for the rendering segment. Status of
record stays in [roadmap.md](../roadmap.md) and design in
[graphics-design.md](../graphics-design.md); this file only says *who owns
what* and *what is queued*. Update it in the same change that moves an
item.

## Owned crates

| Crate | Tier | Role |
|---|---|---|
| `meridian-graphics-core` | 6 | Scene vocabulary, extraction, culling, render graph, lighting, shadows, bloom, submission bridge (`SceneRenderer`). |
| `meridian-graphics-driver` | 3 | RHI: device, surface/swapchain, buffers, textures, shaders, render pipelines, command buffers. No scene knowledge. |
| `meridian-gpu-driver` | shared | `wgpu` mechanics under both `graphics-driver` and `compute-driver` ([ADR 011](../adr/011-shared-gpu-driver-crate.md)). Co-owned with team/core (compute side). |

Owned docs: `docs/graphics-design.md`, ADR 010 (winit), ADR 011 (shared
GPU driver), the graphics sections of `roadmap.md`.

## Interfaces with other segments

- **team/core** — `gac-core` (`Motor3::to_mat4`, `Projection`, `Shape`,
  `ConvexVolume`), `ecs-core` (extraction source), `resource-core` handles,
  `compute-runtime`/`gac-compute`, `engine-core` (`Runtime::tick`).
  Rendering consumes these; it never copies their logic.
- **team/ui-ux** — future `ui-core` emits `Scene2D`/`Scene3D` renderables;
  rendering owns no widgets, layout or text shaping.
- **team/data** — `asset-core` decoders (images now, glTF later) feed
  `TextureRegistry`/`MeshRegistry`; rendering owns no decoders.
- **team/physics** — shares only `gac-core` shapes and the
  `examples/` windowed apps (`physic_figures`).
- **team/standards** — Craftbook rules apply once published; see "Standards
  gaps" below.

Skills to invoke here: `meridian-subsystem-core-keeper` (always),
`meridian-gac-math-keeper` (camera/frustum/shadow-camera math),
`meridian-compute-driver-keeper` (anything touching `gpu-driver`),
`meridian-crate-boundary-keeper` (any `Cargo.toml` edit).

## Current state (2026-10-27)

Real and tested on CPU: `Camera`, `Frustum` (Gribb/Hartmann vs any
`Shape`), `RenderGraph::execution_order`, `extract_scene3d`/`cull_scene3d`,
`directional_shadow_camera`. Real on GPU, proven by the windowed examples
`magic_figures`/`physic_figures`: textured/unlit/lit pipelines (Blinn-Phong,
directional + point, `MAX_LIGHTS = 4`), hemisphere ambient, one directional
shadow map with PCF, non-HDR bloom. `graphics-driver` has headless and
windowed (`Surface`) devices. 30 `#[test]`s across the three crates.

## Backlog

Not started; ordered by suggested priority. Items wait for an explicit task.

1. **Docs drift (small, do first).** `graphics-design.md` step 3 and the
   `submission.rs` module doc still call shadow mapping future work, while
   `roadmap.md` and the code say it is real. `roadmap.md` also still has an
   older paragraph saying extraction/lighting are scaffolds and the driver
   is headless-only. Reconcile all three.
2. **Material flags the bridge ignores (3D).** `BlendMode::AlphaBlend` and
   `always_on_top` exist on `Material` but every draw is opaque and
   depth-tested. Needs a sorted transparent pass and a no-depth overlay
   pass in the render graph.
3. **`Scene2d` path — done (2026-10-27).** `sprites/` in
   `graphics-core`; see graphics-design.md step 5. Follow-ups: bind-group
   cache, scissor/clip rectangles for UI, atlas packing, glyph-atlas
   textures once team/ui-ux has text, composing `[Scene3d, Scene2d]` in
   `meridian-sdk`.
4. **Per-instance data / instancing.** Vertices are baked to world space
   and textured bind groups rebuilt every frame. Add a per-instance uniform
   or storage buffer, then indirect draw and compute culling (GPU-driven
   direction in graphics-design.md).
5. **Per-material roughness/specular.** Shininess is hard-coded to 32.
6. **Render-graph resource lifetime.** Ordering is real; transient target
   allocation/aliasing is not.
7. **`Runtime::tick` integration.** Blocked on an architecture decision:
   `engine-core` must not depend on `graphics-driver`. Needs an ADR
   (likely an app-side presenter trait owned by `meridian-sdk`).
8. **Larger, ADR-gated:** HDR (`Rgba16Float` + tone mapping), shadow
   cascades/frustum fitting, point-light shadows, IBL, glTF meshes (with
   team/data), animation (no vocabulary yet), concrete backend crates.

## Standards gaps

Craftbook (team/standards) has no published rules yet, so this list is a
placeholder to fill once it lands. Known candidates to check against:

- 27 `unwrap`/`expect`/`panic!` sites across the three crates (tests
  included); audit which are reachable from public API.
- `submission.rs` is 1734 lines; likely to exceed any module-size rule.
- GPU paths are proven only by examples; no headless GPU smoke test in CI.

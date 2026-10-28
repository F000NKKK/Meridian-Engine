# Segment: rendering (`team/rendering`)

Ownership note and working backlog for the rendering segment. Status of
record stays in [roadmap.md](../roadmap.md) and design in
[graphics-design.md](../graphics-design.md); this file only says *who owns
what* and *what is queued*. Update it in the same change that moves an
item.

## Owned crates

| Crate | Tier | Role |
|---|---|---|
| `meridian-rendering-domain` | 6 | Scene vocabulary, extraction, culling, render graph, lighting, shadows, bloom, submission bridge (`SceneRenderer`). |
| `meridian-rendering-wgpu` | 3 | RHI: device, surface/swapchain, buffers, textures, shaders, render pipelines, command buffers. No scene knowledge. |
| `meridian-gpu-wgpu` | shared | `wgpu` mechanics under both `rendering-wgpu` and `compute-wgpu` ([ADR 011](../adr/011-shared-gpu-driver-crate.md)). Co-owned with team/core (compute side). |

Owned docs: `docs/graphics-design.md`, ADR 010 (winit), ADR 011 (shared
GPU driver), the graphics sections of `roadmap.md`.

## Interfaces with other segments

- **team/core** — `gac` (`Motor3::to_mat4`, `Projection`, `Shape`,
  `ConvexVolume`), `ecs` (extraction source), `resource` handles,
  `compute-app`/`gac-wgpu`, `runtime-app` (`Runtime::tick`).
  Rendering consumes these; it never copies their logic.
- **team/ui-ux** — future `ui-domain` emits `Scene2D`/`Scene3D` renderables;
  rendering owns no widgets, layout or text shaping.
- **team/data** — `asset-domain` decoders (images now, glTF later) feed
  `TextureRegistry`/`MeshRegistry`; rendering owns no decoders.
- **team/physics** — shares only `gac` shapes and the
  `examples/` windowed apps (`physic_figures`).
- **team/standards** — Craftbook rules apply once published; see "Standards
  gaps" below.

Skills to invoke here: `meridian-subsystem-core-keeper` (always),
`meridian-gac-math-keeper` (camera/frustum/shadow-camera math),
`meridian-compute-driver-keeper` (anything touching `gpu-wgpu`),
`meridian-crate-boundary-keeper` (any `Cargo.toml` edit).

## Current state (2026-10-27)

Real and tested on CPU: `Camera`, `Frustum` (Gribb/Hartmann vs any
`Shape`), `RenderGraph::execution_order`, `extract_scene3d`/`cull_scene3d`,
`directional_shadow_camera`. Real on GPU, proven by the windowed examples
`magic_figures`/`physic_figures`: textured/unlit/lit pipelines (metallic-roughness,
directional + point, `MAX_LIGHTS = 4`), hemisphere ambient, one directional
shadow map with PCF in four fitted cascades (ADR 037), HDR bloom and tone mapping (ADR 034). `rendering-wgpu` has headless and
windowed (`Surface`) devices. 30 `#[test]`s across the three crates.

## Backlog

Not started; ordered by suggested priority. Items wait for an explicit task.

1. **Docs drift — done (2026-10-27).** Shadow mapping, the windowed driver
   and the Motor3 batching cost are reconciled across `graphics-design.md`
   and `roadmap.md`.
2. **Material flags (3D) — done (2026-10-27).** `BlendMode::AlphaBlend` and
   `always_on_top` pick a `DrawState`; see graphics-design.md step 3.
   Follow-ups: per-triangle sorting or order-independent transparency, and
   keeping translucent surfaces out of bloom.
3. **`Scene2d` path — done (2026-10-27).** `sprites/` in
   `rendering-domain`; see graphics-design.md step 5. Clip rectangles are done
   too (`ClipSpan`/`ClipRect`, scissor per run). Bind groups are shared per
   texture within a scene. Glyph atlas packing lives in `text-domain`; a
   general RGBA sprite atlas waits for a consumer. Composition `[Scene3d, Scene2d]` is
   done (`FrameRequest::overlays`, `OverlayPass`, `compose_frame`).
4. **Instancing — done (2026-10-27).** Meshes upload once, renderables
   become instance records, opaque ones batch into instanced draws; see
   graphics-design.md step 3. Follow-ups: skip rewriting unchanged
   instances, then indirect draws and compute culling (GPU-driven
   direction in graphics-design.md).
5. **Per-material surface — done.** `shininess`/`specular` (2026-10-27) became
   `roughness`/`metallic` with a GGX shader (2026-10-28) ([ADR 032](../adr/032-metallic-roughness-materials.md)).
6. **Render-graph resource lifetime — planning done (2026-10-28).**
   `RenderGraph::resource_lifetimes` and `plan_allocations` (CPU, tested) say
   which transient targets can share one allocation. The real renderer
   (`SceneRenderer`, bloom) does not build a `RenderGraph` yet, so no backend
   consumes the plan; wiring a pool in `rendering-wgpu` waits for that.
7. **`Runtime::tick` integration — done.** `meridian_sdk::RenderStage`
   presents through the `FrameRecorder` port, so `runtime-app` never names
   rendering; see [ADR 029](../adr/029-render-stage-presents-through-the-frame-recorder-port.md).
8. **glTF static models — done (2026-10-28).** See
   [ADR 030](../adr/030-gltf-static-models-through-an-asset-adapter.md);
   decoder adapter `meridian-asset-gltf`, `AssetCache::load_model_gltf`.
   JPEG textures (`meridian-asset-jpeg`), external-file `.gltf` and `u32`
   mesh indices are done; checked on Khronos DamagedHelmet, Duck and Sponza
   with the `gltf_viewer` example. Follow-ups: skins and animation.
9. **Metallic-roughness materials — done (2026-10-28).**
   [ADR 032](../adr/032-metallic-roughness-materials.md): `roughness`/`metallic`
   factors, the glTF metallic-roughness map (linear upload), GGX shading.
   Normal, occlusion and emissive maps followed in
   [ADR 033](../adr/033-normal-occlusion-and-emissive-maps.md) (derivative
   tangent frame; emissive-mapped surfaces do not bloom yet).
10. HDR scene target, HDR bloom and tone mapping:
   [ADR 034](../adr/034-hdr-scene-target-and-tone-mapping.md).
11. Image-based lighting from a prefiltered environment (procedural sky for
   now): [ADR 035](../adr/035-image-based-lighting.md).
12. Radiance `.hdr` environment loader: [ADR 036](../adr/036-radiance-hdr-environment-loader.md).
13. Cascaded shadow maps: [ADR 037](../adr/037-cascaded-shadow-maps.md).
14. Cube-map environment source: [ADR 038](../adr/038-cube-map-environment-source.md).
15. Visible environment backdrop: [ADR 039](../adr/039-visible-environment-backdrop.md).
16. Per-cascade caster culling and cascade blending: [ADR 040](../adr/040-per-cascade-casters-and-blending.md).
17. **Larger, ADR-gated:** point-light
   shadows, skinned glTF and
   animation (no vocabulary yet), concrete backend crates.

## Standards gaps

Craftbook (team/standards) has no published rules yet, so this list is a
placeholder to fill once it lands. Known candidates to check against:

- 27 `unwrap`/`expect`/`panic!` sites across the three crates (tests
  included); audit which are reachable from public API.
- `submission.rs` is 1734 lines; likely to exceed any module-size rule.
- GPU paths are proven only by examples; no headless GPU smoke test in CI.

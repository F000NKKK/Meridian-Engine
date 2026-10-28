# ADR 049: headless rendering and per-pass GPU timing

## Status

Accepted. Builds on [ADR 034](034-hdr-scene-target-and-tone-mapping.md) (the HDR
scene target and the tone-mapping resolve into the final target) and
[ADR 011](011-shared-gpu-driver-crate.md) (the shared GPU device crate).

## Context

Everything that draws needed a window: the viewers, a benchmark, an editor
viewport, asset thumbnails and captured video all want frames without one, and a
benchmark measured through a swapchain reports the display's vsync, not the
renderer. Nothing could say how long a pass took on the GPU either.

## Decisions

**D1 — a surface may be a texture.** `Surface::offscreen(&device, w, h)` renders
into an `Rgba8UnormSrgb` texture instead of a swapchain, and `read_pixels` returns
tightly packed RGBA8 rows (top first). `SurfaceFrame::present` does nothing for it.
Because the recorder only ever asks its surface for a format and a frame view,
`GpuFrameRecorder` is unchanged and the whole pipeline (shadows, sky, bloom, tone
mapping, overlays) runs as it does in a window. `resize` replaces the texture.

**D2 — the SDK exposes it.** `GraphicsSession::headless(w, h)` opens a headless
`Device::new()` plus an offscreen surface; `render_to_pixels(&scene, clear)` draws
and reads back. A device is reused across frames. Adapters without a GPU (a CI
box with no Vulkan) still fail at device creation, as before.

**D3 — passes are timed with timestamp queries.** The device requests
`TIMESTAMP_QUERY` when the adapter has it. `PassProfiler` owns one query pair per
pass (64 per frame), resolves them with the frame's command buffer and `collect`
reads them back (it waits for the GPU, so profiling is opt-in:
`GpuFrameRecorder::enable_profiling`). Passes carry a name
(`CommandBuffer::label_next_pass`; a pass without one is named by its kind) and
times of equal names are summed. Without timestamp support `enable_profiling`
returns `false` and no times exist.

**D4 — an upright camera.** `look_at_rotor` is a shortest arc and rolls the camera
as it orbits; `look_at_upright` keeps the right axis horizontal and is what orbit
and editor cameras use.

**D5 — the render benchmark is an example.** `examples/render_bench` draws a fixed
scene (`helmet`, `shadows`, `instances`, `crowd`) headless for N frames on a fixed
clock and reports CPU update, CPU render and each GPU pass (mean, p95, max) as a
table on stderr and one JSON line on stdout. `--dump DIR` writes the frames as
images for encoding video. `MERIDIAN_FIXED_DT` gives the windowed viewers the same
fixed step.

**D6 — an editor renderer on top.** `meridian_sdk::EditorRenderer` owns one headless
session and one sky: `render(&World, &EditorCamera)` returns RGBA8 for every
entity with a transform and a `MeshRenderer`, lit by its `LightSource`s (a default
sun when there are none); `resize` follows the viewport. `thumbnail_material`
(a sphere), `thumbnail_mesh`, `thumbnail_model` (glTF at rest) and
`thumbnail_scene` render a fixed-angle, auto-framed square image of one asset;
`THUMBNAIL_RENDERER_VERSION` is bumped whenever the same asset would look
different, so a cache keyed with it stays valid. `EditorCamera` orbits a target
(`yaw`, `pitch`, `distance`, `fov_y`) and never rolls (D4). `LightSource` (kind
directional, point or spot, colour, intensity) is the light component:
`extract_scene3d` turns it into a `Light` at the entity's frame.

## Consequences

- A software adapter (llvmpipe) reports real pass times, so a baseline exists
  without a GPU, but they say nothing about a hardware GPU; reports name the adapter.
- Editor viewport and thumbnails build on D1/D2 and need no further renderer change.
- Every pass that should appear by name in a profile must call `label_next_pass`.

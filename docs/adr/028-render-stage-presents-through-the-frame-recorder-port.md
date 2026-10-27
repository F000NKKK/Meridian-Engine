# ADR 028: Presenting a frame is a `Stage` in the sdk, through the `FrameRecorder` port

## Status

Accepted. Records a decision already carried out in code (`RenderStage`,
`FrameRecorder`) that the roadmap still described as open. Builds on
[ADR 019](019-ports-in-domain-drivers-as-adapters.md) and
[ADR 025](025-wave-6-runtime-and-composition-root.md) D4.

## Context

`docs/roadmap.md` and `docs/graphics-design.md` listed "`Runtime::tick`
integration" of rendering as blocked: presenting a frame needs a windowed
`Device`/`Surface`, which is driver state, and `runtime-app` must not depend
on `rendering-wgpu`. The question was where the presenting code lives.

## Decisions

**D1 — `runtime-app` stays ignorant of rendering.** It knows `Stage`,
`Runtime::add_stage` and `Runtime::resize_all`. It names no window, device
or swapchain, and takes no edge to `rendering-wgpu` or `rendering-app`.

**D2 — the port is `rendering-app::FrameRecorder`.** `record_frame` and
`resize` are the whole surface a presenter needs. `rendering-wgpu` implements
it (`GpuFrameRecorder`); `testing::FakeFrameRecorder` and a contract suite
cover it. `RenderSession<R: FrameRecorder>` is the application-facing driver
of the port. No second presenter trait is introduced: a trait owned by the
sdk would duplicate this one.

**D3 — the stage is composition glue and lives in `meridian-sdk`.**
`RenderStage` wraps a `GraphicsSession` (a `RenderSession` over the wgpu
recorder) and a window, builds a `Scene3d` (and optional `Overlay2d`s) from
`Shared` handles each tick, and presents. The sdk is the layer allowed to
name adapters (ADR 025 D4), so the wgpu edge stays there. Its dependencies
on other stages (usually physics) are declared at `add_stage`, so the
runtime guarantees the state is ready before the frame is drawn.

**D4 — an application may bypass `Runtime`.** `render_frame` and
`compose_frame` are the same sequence as plain functions, for programs whose
other subsystems do not fit the stage model (`magic_figures` hand-assembles
its binaural audio rig). That is a documented escape hatch, not a gap.

## Consequences

- The roadmap and graphics-design items that call this blocked are
  reconciled in the same change; the rendering backlog item is closed.
- `physic_figures` is the reference: physics stage, then `RenderStage`
  depending on it, driven by `Runtime::tick`/`tick_fixed`.
- A future non-wgpu backend implements `FrameRecorder` and needs no change in
  `runtime-app`. Only the sdk's `GraphicsSession` alias names the adapter.

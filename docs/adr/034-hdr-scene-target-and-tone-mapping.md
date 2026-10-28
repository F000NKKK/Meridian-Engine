# ADR 034: HDR scene target and tone mapping

## Status

Accepted. Builds on the lighting of [ADR 032](032-metallic-roughness-materials.md)
and the maps of [ADR 033](033-normal-occlusion-and-emissive-maps.md); both
noted that the output clips at 1.0.

## Context

Every colour the renderer wrote landed in an 8-bit sRGB target, so a lit
surface brighter than 1.0 clipped to flat white: a strong light on a pale
material lost its shading, a specular lobe turned into a disc, and bloom
could only glow what the 8-bit bright pass had already clamped. Image-based
lighting (next) needs a target that can hold sums of many bright terms.

## Decisions

**D1 — the 3D passes draw into an `Rgba16Float` scene target.**
`ColorFormat::HdrRgba16F` joins `SrgbRgba8`. `GpuFrameRecorder` owns one scene
texture of the swapchain's size, rebuilt on resize, and the main pass draws
into it with the existing depth buffer. The sixteen main pipelines are built
for that format: `SceneRenderer::new` takes a `ColorFormat` instead of a
`Surface`, and `Device::create_scene_pipeline` likewise. Shader outputs stay
linear; nothing is clamped until D2.

**D2 — a tone mapping pass resolves the scene into the swapchain.**
`TonemapPass` draws one full-screen triangle that reads the scene texture,
multiplies it by an exposure and applies a curve, writing the swapchain's sRGB
view, so the display transfer is the hardware's. It runs after bloom and
before the overlays, so UI and sprites are never tone mapped or blurred.

**D3 — `TonemapConfig` is domain data.** `TonemapConfig { exposure, operator }`
and `TonemapOperator { Clamp, Reinhard, Aces }` live in rendering-domain beside
`BloomConfig`. The default is exposure 1.0 with ACES (Narkowicz's fit): a
filmic curve with midtone contrast and a soft highlight shoulder. `Clamp`
reproduces the old look for a caller who wants it. `GpuFrameRecorder::set_tonemap`
changes the config between frames; there is no auto exposure.

**D4 — bloom becomes HDR.** Its bright, ping and pong targets use
`HdrRgba16F` and the composite pipeline is additive onto the scene texture, so
an emissive value above 1.0 now glows in proportion and highlights that were
clamped before feed the blur unclamped. `BloomPass::new` no longer takes a
`Surface`, and `BloomInput` names the scene texture as its target.

## Consequences

- The picture changes: with ACES at exposure 1.0 midtones are a little
  contrastier and highlights roll off instead of clipping, and the clear colour
  is tone mapped like everything else. Examples keep their code; a scene that
  wants the old values sets `TonemapOperator::Clamp`.
- One more full-resolution texture (8 bytes per pixel) and one more
  full-screen draw per frame.
- The bloom bright pass still redraws geometry and reads only the plain
  emissive slot; emissive maps bloom through a second pipeline (ADR 042).
- No auto exposure, no per-channel white point, no colour grading. The
  operator is an enum so they can be added later without touching the scene
  passes.
- Sprites and overlays still draw into the 8-bit swapchain after tone mapping;
  only the 3D scene is HDR.

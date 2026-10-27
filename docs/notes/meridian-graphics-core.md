# `meridian-graphics-core` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `bloom`

**Pipeline, once per frame a bloom-enabled view renders:**
1. [`SceneRenderer::draw`] — the ordinary lit/unlit scene, straight
into the real render target (swapchain or an offscreen color
texture).
2. [`BloomPass::apply`]'s bright pass — the *same* `DrawBuffers`
redrawn through this module's own emissive-extraction pipelines
(built here, not on `SceneRenderer` — see [`BloomPass::new`]'s doc
comment for why) into an offscreen "bright" texture. A material
with `emissive = [0,0,0]` (the default) contributes solid black
here — harmless once additively composited in step 4 — so no
per-material bloom flag or threshold is needed; `emissive` itself
*is* the mask.
3. Two blur passes (horizontal, then vertical — the standard
separable-Gaussian trick: an `N*N` blur becomes two `N`-tap passes)
ping-ponging between two same-sized offscreen textures, each a
full-screen-triangle draw sampling the previous pass's texture.
4. The composite pass: the blurred bright texture, scaled by
[`BloomConfig::intensity`], additively blended onto the *same*
view the main pass (step 1) already drew into — via
`CommandBuffer::begin_render_pass_loaded`, which preserves existing
contents instead of clearing them.

**Deliberately not HDR.** Every offscreen texture here (bright/ping/
pong) is [`ColorFormat::SrgbRgba8`], the same format `TextureRegistry`
uses for uploaded textures — an emissive value is whatever a
`Material` sets (no bloom-specific tone mapping or exposure control),
clamped to `[0,1]` like every other color in this bridge. A true HDR
pipeline (`Rgba16Float` intermediate targets, exposure/tone-mapping)
is real future work if a scene ever needs emissive values above `1.0`
to look right — it would only mean adding a variant to
`graphics-driver::ColorFormat` and passing it through, not a redesign
here; the visible glow this produces today does not depend on it.

**Fixed at construction size**, matching the swapchain/target it was
built for — resize by constructing a new [`BloomPass`] (cheap: a
handful of small offscreen textures and three tiny pipelines), the
same "rebuild on resize" pattern `DepthTexture` already uses in every
windowed example.

## `submission`

**No per-instance uniform, no GPU-side instancing yet.** A render
pass here has exactly one shared uniform buffer (view-projection,
camera position, ambient, lights) per view — there is no dynamic-
offset or per-draw uniform mechanism in `graphics-driver` today, and
writing a *shared* uniform once per draw would race (every draw only
executes once the whole command buffer is submitted, by which point
the last write wins for *all* of them — the same footgun the
soft-body examples' buffer-lifetime comments call out). So each
[`Renderable3D`]'s world-space position/normal, its UV (if textured)
and its material's tint/emissive are baked into a fresh vertex buffer
at [`SceneRenderer::prepare`] time, once per instance, per frame —
correct today, and exactly the CPU-side cost docs/graphics-design.md's
"GPU-driven rendering" section already names as the long-term thing
to fix via instancing/indirect draw. Per-textured-draw bind groups
are similarly rebuilt every frame rather than cached per texture —
the same "correct now, batch later" trade-off.

**Lights are capped at [`MAX_LIGHTS`]** (a fixed-size WGSL uniform
array, not a dynamically-sized storage buffer — keeping the uniform
shape simple until a scene actually needs more): extra lights beyond
the cap are dropped, logged once per [`SceneRenderer::prepare`] call
via `meridian_foundation::log_warn!`, not silently ignored.

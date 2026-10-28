# `meridian-examples` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `examples/magic_figures`

Rendering goes entirely through `rendering-domain`'s submission bridge
(`meridian_sdk::GraphicsSession` bundles `SceneRenderer`/`BloomPass`/the
three registries — the same base `physic_figures` builds): the floor
is a real lit (metallic-roughness) material under one directional light +
ambient; each shape is `unlit` + `emissive` in its own color, so it
always reads as glowing regardless of scene lighting, and
`BloomPass` turns that emissive value into a real halo — see
`meridian_rendering_domain::bloom`'s module doc for the shader details
(separable Gaussian blur, additive composite).

A free-fly camera (WASD + mouse, cursor grabbed on launch, Escape
toggles the grab) doubles as the audio listener — same `Motor3`
frame, same local-forward-`+X` convention `audio-domain::Listener`
uses — so all three tracks pan/attenuate through
`audio-domain::BinauralRenderer` as you fly around, mixed down to one
output stream in a single `render` call per frame (all three source
files happen to share a sample rate, so one `BinauralRenderer`/
`AudioStream` pair suffices — see [`load_music_tracks`]). The
audio runs as an `AudioStage` on a `Runtime`: [`MusicRig`] builds an
`AudioScene` (one looping `Voice` per shape, `BinauralRenderer`, `Declicker`)
and each frame only sets the listener and voice emitters before
`Runtime::tick`. There are no rigid bodies (pure kinematic orbit motion).
Orbit/spin motion itself stays genuine per-frame Rust logic (the DSL
describes composition, not behavior) — only each shape's *fixed*
identity comes from the scene file.

This example depends on `meridian-sdk` alone (plus `tokio`, for its
own async GPU/audio-device handshakes) — every type below is reached
through `meridian_sdk`'s re-exports, never through
`meridian-gac`/`meridian-audio-domain`/`meridian-rendering-domain`/
etc. directly.

Run with:
./build.sh run magic_figures

## `examples/gltf_viewer`

Loads the glTF model named on the command line through
`GraphicsSession::load_model_gltf` (ADR 030): one mesh and one material per
model part, a directional sun and a sky ambient, and a camera orbiting the
model's bounding sphere. The second argument is the camera distance in radii;
below 1 the camera is inside the bounds, which is how Sponza is viewed.
A skinned model plays its first animation (`MERIDIAN_ANIMATION=<n>` picks
another), posed by the renderer's skinning kernel (ADR 047). The models are not
part of the repository (Khronos glTF-Sample-Assets).

Run with:
./build.sh run gltf_viewer -- path/to/model.glb

## `examples/render_bench`

The render benchmark (ADR 049): draws one fixed scene headless (no window, no
vsync) for N frames on a fixed clock and reports the CPU time of updating and of
recording/submitting, and the GPU time of every pass from timestamp queries.
Scenes: `helmet` (any glTF via `--model`, default `examples/assets/models/DamagedHelmet.glb`,
IBL and bloom), `shadows` (floor, forty props, sun, point and two moving spot
lights), `instances` (4096 cubes), `crowd` (100 animated skinned models, default
`examples/assets/models/CesiumMan.glb`). The table goes to stderr, one JSON line to stdout
(and to `--json FILE`); `--dump DIR` writes every measured frame as a `.ppm`.
The resolution is `MERIDIAN_WINDOW=<w>x<h>` (default 1920x1080).

Run with:
./build.sh run render_bench -- shadows --frames 300 --json bench.json

## `examples/ui_scroll`

A `ScrollViewer` (ADR 018) over a stack of twelve coloured rows, its
`VerticalOffset` driven by the clock. Each frame the tree is laid out,
`ui_scene2d::extract_clipped` turns it into sprites plus clip spans, and both go
into an `Overlay2d` that `compose_frame` draws over the 3D frame; the sprite
pass applies the spans as scissor rects (`batch_scene2d_clipped`), so only the
rows inside the viewer's window show. Backgrounds only: text needs a font file
and a glyph atlas upload, which the example leaves out.

Run with:
./build.sh run ui_scroll

## `examples/physic_figures`

**Scene composition itself lives in
`assets/scenes/physic_figures.meui`**, parsed through
`meridian_sdk::dsl` (via this workspace's shared
`meridian_examples::scene_loader`) — every entity's position, mesh
shape, texture and collider is data, not Rust code; this file only
reads it once ([`load_scene`]) and turns it into real physics bodies
([`PhysicsRig::new`]) and renderables (the `RenderStage`'s
`build_scene` closure, registered in
[`AppHandler::on_ready`]). What stays genuine Rust logic (per this
workspace's rule that the DSL describes composition, not behavior):
the fixed-timestep accumulator, the free-fly camera, and each
shape's own render-frame quirk (the floor mesh is a thin visual quad
rendered at identity while its *collider* is a thicker slab; the
pyramid mesh's origin is its base center while its *collider* is
centered on the body — both disclosed per-shape offsets in
[`mesh_render_frame`], the same simplification the hand-written
version had, just no longer duplicated as four hardcoded literals).

`physics-domain` only has two collider shapes today, `Sphere` and
`Cuboid` — there is no dedicated pyramid collider, so the DSL scene
gives the pyramid entity a `Cuboid` collider roughly bounding its
mesh (it settles and rests like a box).

Shares its base with `magic_figures` (`meridian_sdk::scene`'s
`GraphicsSession`): same mesh builders, same lighting model. No bloom
emissive glow here beyond the visible "sun" sphere — these are
ordinary lit, textured physics props, not the "magic" glowing
shapes.

This example depends on `meridian-sdk` alone (plus `tokio`, for its
own async GPU-device handshake) — every type below is reached
through `meridian_sdk`'s re-exports, never through
`meridian-gac`/`meridian-physics-domain`/`meridian-rendering-domain`/
etc. directly.

Run with:
./build.sh run physic_figures

# `meridian-examples` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `examples/magic_figures`

Rendering goes entirely through `graphics-core`'s submission bridge
(`meridian_sdk::GraphicsSession` bundles `SceneRenderer`/`BloomPass`/the
three registries — the same base `physic_figures` builds): the floor
is a real Blinn-Phong-lit material under one directional light +
ambient; each shape is `unlit` + `emissive` in its own color, so it
always reads as glowing regardless of scene lighting, and
`BloomPass` turns that emissive value into a real halo — see
`meridian_graphics_core::bloom`'s module doc for the shader details
(separable Gaussian blur, additive composite).

A free-fly camera (WASD + mouse, cursor grabbed on launch, Escape
toggles the grab) doubles as the audio listener — same `Motor3`
frame, same local-forward-`+X` convention `audio-core::Listener`
uses — so all three tracks pan/attenuate through
`audio-core::BinauralRenderer` as you fly around, mixed down to one
output stream in a single `render` call per frame (all three source
files happen to share a sample rate, so one `BinauralRenderer`/
`AudioStream` pair suffices — see [`load_music_tracks`]). This
example doesn't use `meridian_sdk::Runtime` at all — no rigid bodies
(pure kinematic orbit motion), and [`MusicRig`]'s real per-sample
`BinauralRenderer` synthesis doesn't reduce to a `Runtime::Stage`
cleanly either: its actual inputs (each orbiting shape's position,
the free-fly camera's pose) are plain application state here, not
anything `Runtime`'s `PhysicsSubsystem`/`AudioSubsystem` own — see
`meridian_engine_core`'s own module doc for the same reasoning.
Orbit/spin motion itself stays genuine per-frame Rust logic (the DSL
describes composition, not behavior) — only each shape's *fixed*
identity comes from the scene file.

This example depends on `meridian-sdk` alone (plus `tokio`, for its
own async GPU/audio-device handshakes) — every type below is reached
through `meridian_sdk`'s re-exports, never through
`meridian-gac-core`/`meridian-audio-core`/`meridian-graphics-core`/
etc. directly.

Run with:
./build.sh run magic_figures

## `examples/physic_figures`

**Scene composition itself lives in
`assets/scenes/physic_figures.mel`**, parsed through
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

`physics-core` only has two collider shapes today, `Sphere` and
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
`meridian-gac-core`/`meridian-physics-core`/`meridian-graphics-core`/
etc. directly.

Run with:
./build.sh run physic_figures

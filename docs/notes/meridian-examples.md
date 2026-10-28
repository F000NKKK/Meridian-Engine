# `meridian-examples` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `examples/magic_figures`

Rendering goes entirely through `rendering-domain`'s submission bridge
(`meridian_sdk::GraphicsSession` bundles `SceneRenderer`/`BloomPass`/the
three registries — the same base `physic_figures` builds): the floor
is a real Blinn-Phong-lit material under one directional light +
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
The models are not part of the repository (Khronos glTF-Sample-Assets).

Run with:
./build.sh run gltf_viewer -- path/to/model.glb

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

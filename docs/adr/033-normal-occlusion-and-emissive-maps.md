# ADR 033: normal, occlusion and emissive maps

## Status

Accepted. Completes the glTF metallic-roughness material set begun in
[ADR 032](032-metallic-roughness-materials.md); the glTF side extends
[ADR 030](030-gltf-static-models-through-an-asset-adapter.md).

## Context

With base colour, metalness and roughness, a loaded model still lacks the
detail glTF puts in three more slots: a normal map (surface relief such as a
hose's ribbing), an occlusion map (baked ambient shadow in creases) and an
emissive map with a factor (lit panels, lamps). Without them a model looks
flat next to its source.

## Decisions

**D1 — three slots on `Material`.** `normal: Option<TextureHandle>` with
`normal_scale` (glTF's `scale`, default 1), `occlusion: Option<TextureHandle>`
with `occlusion_strength` (default 1) and `emissive_map: Option<TextureHandle>`
beside the existing `emissive` factor, which multiplies the map. The normal
and occlusion maps are linear data (`TextureStore::upload_linear`); the
emissive map is colour (`upload`). `MaterialMaps` in rendering-domain names the
five textures of a material and replaces the albedo and metallic-roughness
fields of `BatchKey`, so two draws batch only when they bind the same maps.

**D2 — the tangent frame comes from screen-space derivatives.** The shader
builds the tangent from the derivatives of world position and UV and the
bitangent as `cross(normal, tangent)`, glTF's convention for UV with `v`
pointing down the image. Meshes carry no tangent attribute, the mesh vertex
(32 bytes) and the glTF reader stay as they are, and models with a `TANGENT`
attribute are shaded the same way. The cost is that the frame is per-pixel and
flat across a triangle's edge-on or UV-degenerate texels, where the shader
falls back to the mesh normal; mirrored UVs are not detected. A tangent
attribute can replace this later without changing `Material`.

**D3 — occlusion dims ambient only.** `occlusion` (red channel) scales the
hemisphere ambient term, diffuse and specular, by `mix(1, texel, strength)`;
direct light is untouched, as in glTF.

**D4 — the instance record carries the strengths and the mapped emissive.**
The 100-byte record grows to 120: `normal_scale` and `occlusion_strength`
(two floats) and an emissive factor used only with a map. A material with an
emissive map puts its factor there and leaves the plain emissive slot zero.
That plain slot is the one bloom's emissive pass reads, and bloom does not
sample textures, so **a material with an emissive map does not bloom**; one
without a map blooms as before. Sampling the map in the bloom pass is a
follow-up.

**D5 — one bind group layout, neutral textures.** A lit textured draw binds
all five maps at `@binding(1)` (albedo) and `@binding(5..=8)` (metallic-
roughness, normal, occlusion, emissive), all read through the albedo's sampler.
A map a material lacks is replaced by a 1x1 neutral texture owned by the scene
renderer: white colour (albedo, emissive), white linear (metallic-roughness,
occlusion) and the flat normal `(0.5, 0.5, 1.0)`. The pipeline count and the
colour-only pipelines are unchanged. Unlit draws ignore the new maps.

**D6 — glTF.** `ModelMaterial` gains `normal_image`, `normal_scale`,
`occlusion_image`, `occlusion_strength`, `emissive` and `emissive_image`. The
adapter reads them; the sdk loader uploads each image once per way of reading
it (colour or linear), since one image can serve both.

## Consequences

- Draws with maps sample five textures; the per-stage limit of sixteen leaves
  room for image-based lighting.
- `Material` gains five fields; code that builds one with `..Default::default()`
  is unaffected.
- Emissive-mapped surfaces do not glow through bloom until the bloom pass
  samples the map.

# ADR 032: metallic-roughness materials

## Status

Accepted. Supersedes the "a PBR upgrade changes the shader, not this
vocabulary" line in `Light`'s doc and the `shininess`/`specular` pair of
[graphics-design.md](../graphics-design.md). Builds on
[ADR 030](030-gltf-static-models-through-an-asset-adapter.md), which reads only
the base colour of a glTF material.

## Context

A material has a Blinn-Phong exponent (`shininess`) and a highlight strength
(`specular`). glTF models describe their surface as metallic-roughness: two
factors and one texture whose green channel is roughness and blue channel is
metalness. Loaded models (ADR 030) therefore render with the base colour only
and a highlight chosen by the example, so a helmet's metal and its rubber look
alike. The renderer has no HDR target and no environment maps (both parked), so
the model has to be useful without them.

## Decisions

**D1 — the material vocabulary is roughness and metallic.**
`rendering-domain::Material` drops `shininess` and `specular` and gains
`roughness: f32` (0 mirror-smooth, 1 fully rough, default 0.5) and
`metallic: f32` (0 dielectric, 1 metal, default 0.0), both clamped to `[0, 1]`
where the renderer reads them. Two knobs replace two knobs, so the type does
not grow; the old pair described one lobe's width and height, and the new pair
describes the surface. The default keeps the look of a default material close
to what it was (a broad, faint highlight). In-repo users (`magic_figures`,
`gltf_viewer`, tests) are migrated in the same change.

**D2 — the maps.** `Material` gains `metallic_roughness: Option<TextureHandle>`
in glTF's layout: roughness in the green channel, metalness in the blue
channel, each multiplied with the material's factor, so a model with a map and
factors 1.0 (glTF's default) reads exactly the map. The texture is **linear
data**, not colour: it is uploaded without an sRGB decode. `TextureStore`
gains `upload_linear(&ImageAsset)` beside `upload`, and the GPU registry keeps
the format with the texture. Without a map the factors alone apply.
Normal, occlusion and emissive maps are not part of this ADR.

**D3 — shading.** The lit shaders replace Blinn-Phong with the metallic-
roughness model: GGX normal distribution (`alpha = roughness²`), height-
correlated Smith visibility, Schlick Fresnel with `F0 = mix(0.04, base_color,
metallic)`, and a diffuse term `base_color * (1 - metallic)`. Light intensities
keep their meaning (a white diffuse surface facing a light of intensity 1.0 is
fully lit), so the diffuse term has no `1/π` and the specular lobe is scaled by
`π` to match. With no environment lighting, the ambient term is the hemisphere
gradient the scene already has: diffuse ambient uses the surface normal, and a
specular ambient uses the reflected direction with a roughness-damped Fresnel,
so a metal reflects the sky and ground colours instead of going black. This is
a disclosed stand-in for image-based lighting, which stays parked; when it
lands it replaces the ambient lookups and nothing else.

**D4 — the instance record.** The per-instance `gloss` pair (shininess,
specular) becomes `surface` (roughness, metallic); the 100-byte record keeps
its size. Batching: the metallic-roughness texture joins the albedo in the
batch key, because a draw binds both.

**D5 — glTF.** `asset-domain::ModelMaterial` gains `metallic: f32` and
`roughness: f32` (glTF's defaults are 1.0 and 1.0) and
`metallic_roughness_image: Option<usize>`. The adapter reads them; the sdk
loader uploads the image once as linear data and fills the `Material`. The
decoder still never decodes pixels.

**D6 — delivery in two steps.** The factors and the shading (D1, D3, D4, the
factor half of D5) landed first; the map (D2, the image half of D5, the extra
texture binding and the batch key) followed. A lit textured draw always
samples a metallic-roughness texture: a draw without a map binds a 1x1 linear
white one owned by the scene renderer, so the pipeline count stays the same,
and a draw with a map but no albedo binds a 1x1 white albedo.

## Consequences

- Breaking change to `Material` (two fields replaced). Everything in the
  workspace that builds one with `..Default::default()` keeps working; the
  four users of the old fields are migrated.
- A lit textured draw with a map binds two textures and a shadow map, one more
  bind group layout and one more pipeline family, which is the cost of D2.
- Roughness 0 with a point light gives a one-pixel highlight; the shader clamps
  `alpha` from below so a mirror-smooth surface stays finite.
- The output is still not HDR: a bright specular lobe clips. An HDR target is
  parked with bloom's.

# ADR 030: glTF 2.0 static models through an asset adapter

## Status

Accepted. Applies [ADR 024](024-wave-5-ports-and-adapters.md) D5 (a codec is an
adapter, a pure parser stays in the domain) and follows the trigger set by
[ADR 013](013-compressed-audio-codecs.md) ("a concrete asset needs it").

## Context

`ObjDecoder` yields positions and triangle indices only: no normals, UVs,
materials, textures or hierarchy. Content made for games ships as glTF 2.0
(`.gltf`, or `.glb` binary), which carries all of those. The roadmap names glTF
as the next mesh format. The renderer draws static, lit, textured meshes with a
base colour; it has no skeleton or animation vocabulary yet.

## Decisions

**D1 — scope: static models.** v1 reads node hierarchy, mesh primitives
(triangles; `POSITION`, `NORMAL`, `TEXCOORD_0`, indices) and the
metallic-roughness material's base colour factor and base colour texture.
Skins, animations, morph targets, cameras, lights, sparse accessors, texture
transforms and other extensions are ignored, not errors, so a model with them
still loads as its rest pose. A primitive that is not a triangle list is a typed
error. Skinning and animation get their own ADR once the engine has the
vocabulary.

**D2 — the data type is `asset-domain::ModelAsset`.** The domain owns the CPU
result, as it owns `MeshAsset` and `ImageAsset`: a list of `ModelPart`s, one
per primitive instance, each with `positions`, `normals` and `uvs` (empty when
the source has none), `u32` `indices` and a `ModelMaterial` (`base_color:
[f32; 4]`, `base_color_image: Option<usize>`), plus the model's `images` as
`EncodedImage { bytes }`. The node hierarchy is flattened by the decoder: each
part's positions and normals already carry the node's world transform
(normals by the inverse transpose, winding reversed under a mirroring
transform), so a part has no matrix and a node with scale needs nothing
the renderer's rigid `Motor3` cannot hold. Images stay encoded and are
identified by magic bytes: the decoder never decodes pixels, so it needs no
image codec and does not depend on `asset-png`. The composition root decodes
them with its image decoders.

**D3 — the decoder is an adapter, `meridian-asset-gltf`.** It implements
`Decoder<ModelAsset>` over the `gltf` crate (default features off, so no file
loading, no `image`, no `import`). Parsing JSON, accessors, buffer views and
the GLB container is the codec work ADR 024 D5 assigns to an adapter; a
hand-rolled version would duplicate a maintained crate. The adapter depends on
`asset-domain` only.

**D4 — decoding stays separate from loading.** The decoder reads the GLB blob
and `data:` URIs. A `.gltf` that points at an external `.bin` or image file is
`GltfError::ExternalResource(uri)` in v1, because resolving a path is the
loader's job (`asset-app`, `ByteSource`), not the decoder's. A later change may
add a loader step that fetches the named resources and passes them in; the
decoder's input type would grow, its output would not.

**D5 — failures are typed and bounded.** Malformed JSON, a bad accessor,
out-of-range indices and counts that exceed the file all return
`GltfError`, never a panic or an allocation sized by an unchecked field.
Meshes with more than 65 535 vertices are `GltfError::TooManyVertices` until
the renderer takes `u32` indices (the existing `MeshSource` limit; this ADR
does not widen it).

**D6 — the sdk wires it.** `meridian-sdk` names the adapter (ADR 025 D4);
`AssetCache::load_model_gltf` registers each part's mesh and its images with the
existing ports and returns `ModelPiece { mesh, material }` values that an
application places with one frame for the whole model. Missing normals are
filled with `smooth_normals`; missing UVs become zeros. An image the sdk
cannot decode (JPEG, until an adapter exists) is logged and its parts keep
their base colour. The renderer and `rendering-domain` change nothing.

## Consequences

- One new adapter crate and one new workspace dependency (`gltf`), the same
  documented-exception pattern as `png` and `symphonia`.
- `docs/dependency-rules.md` gets the `asset-gltf -> asset-domain` edge.
- Textures embedded in a GLB work end to end; multi-file `.gltf` needs the
  loader extension named in D4.
- Rendering of what glTF can describe beyond this scope (PBR maps, skinning,
  animation, large index counts) stays on the roadmap with the ADR that
  introduces each.

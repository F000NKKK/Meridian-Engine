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
and `data:` URIs and does no file I/O. A `.gltf` that points at an external
`.bin` or image file is handled in two steps: `GltfDecoder::required_uris`
lists the files, the loader fetches them, and `GltfDecoder::decode_with` takes
them as a map keyed by the URI the model uses (`Decoder::decode` supplies none,
so a model naming a file is `GltfError::ExternalResource`). The sdk's
`AssetCache::load_model_gltf` is that loader today: it reads the files from
beside the model, percent-decodes the URIs and refuses any that leave the
model's directory (a scheme, an absolute path, `..`).

**D5 — failures are typed and bounded.** Malformed JSON, a bad accessor,
out-of-range indices and counts that exceed the file all return
`GltfError`, never a panic or an allocation sized by an unchecked field.
The only size bounds are a total vertex count and a node depth, so a node
tree that instances one mesh many times cannot drive allocation. Mesh indices
are `u32` end to end (the renderer no longer has a 65 535-vertex ceiling).

**D6 — the sdk wires it.** `meridian-sdk` names the adapter (ADR 025 D4);
`AssetCache::load_model_gltf` registers each part's mesh and its images with the
existing ports and returns `ModelPiece { mesh, material }` values that an
application places with one frame for the whole model. Missing normals are
filled with `smooth_normals`; missing UVs become zeros. An image the sdk
cannot decode is logged and its parts keep their base colour. JPEG textures
decode through the adapter `meridian-asset-jpeg` (`zune-jpeg`), wired into the
sdk's image sniffing beside PNG and BMP. The renderer and `rendering-domain` change nothing.

## Consequences

- New adapter crates `meridian-asset-gltf` and `meridian-asset-jpeg` and
  workspace dependencies `gltf` and `zune-jpeg`, the same documented-exception
  pattern as `png` and `symphonia`.
- `docs/dependency-rules.md` gets the `asset-gltf -> asset-domain` edge.
- GLB and multi-file `.gltf` models load end to end, checked on the Khronos
  sample assets (DamagedHelmet with JPEG textures, Duck with PNG, Sponza with
  103 parts and 70 external images) with the `gltf_viewer` example under
  lavapipe.
- Rendering of what glTF can describe beyond this scope (PBR maps, skinning,
  animation, large index counts) stays on the roadmap with the ADR that
  introduces each.

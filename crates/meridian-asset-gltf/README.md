# meridian-asset-gltf

glTF 2.0 adapter for `meridian-asset-domain`: decodes `.glb` and self-contained `.gltf` static models on the `gltf` crate.

**Layer and context:** infrastructure · assets

- **For:** `GltfDecoder`, a `Decoder<ModelAsset>` that flattens the node hierarchy into world-space triangle parts with a base colour and base colour texture (ADR 030).
- **Not for:** loading external `.bin` or image files (the loader's job), skins, animations, extensions, the `Decoder` port and `ModelAsset` (those are `meridian-asset-domain`).

## Usage

```rust
use meridian_asset_domain::Decoder;
use meridian_asset_gltf::GltfDecoder;

// Bytes that are neither GLB nor glTF JSON are rejected with a typed error.
assert!(GltfDecoder.decode(b"not a model").is_err());
```

## Depends on / used by

- Depends on: [`meridian-asset-domain`](../meridian-asset-domain), [`meridian-gac`](../meridian-gac)
- Used by: [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

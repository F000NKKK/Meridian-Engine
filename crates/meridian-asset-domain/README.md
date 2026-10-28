# meridian-asset-domain

Loading and decoding of images, meshes, audio, shaders and binary formats. Does not manage ownership or lifetime of loaded data.

**Layer and context:** domain · assets

- **For:** The `Decoder` port, the CPU asset types, format detection by magic bytes, and the pure parsers for BMP, Radiance HDR, WAV, OBJ meshes and shader sources, `ModelAsset`, the multi-part model type glTF decodes to, and Meridian's own asset formats on Substrate Data: `AssetMeta` (`.meta`: stable `AssetId`, importer, settings, dependencies) and the `.mpak` pack (`PackWriter`, `PackReader` over the `ReadAt` port, `upgrade_pack`). See docs/data-formats.md.
- **Not for:** codec libraries or ownership of loaded data: PNG is `meridian-asset-png`, glTF is `meridian-asset-gltf`, JPEG is `meridian-asset-jpeg`, compressed and streaming audio is `meridian-asset-symphonia`, identity is `meridian-resource`, storage `meridian-memory`.

## Usage

This sketch is not compiled as a doc test (it needs a device, a window or a build script).

```rust,ignore
use meridian_asset_domain::{Decoder, ObjDecoder};

let mesh = ObjDecoder.decode(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n")?;
```

## Depends on / used by

- Depends on: [`meridian-diagnostics`](../meridian-diagnostics), and externally `substrate-data` and `thiserror` (the formats' typed errors)
- Used by: [`meridian-asset-fs`](../meridian-asset-fs), [`meridian-asset-png`](../meridian-asset-png), [`meridian-asset-symphonia`](../meridian-asset-symphonia), [`meridian-rendering-wgpu`](../meridian-rendering-wgpu), [`meridian-rendering-app`](../meridian-rendering-app), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

# meridian-asset-core

Loading and decoding of images, meshes, audio, shaders and binary formats. Does not manage ownership or lifetime of loaded data.

**Layer and context:** domain · assets

- **For:** The `Decoder` port, the CPU asset types, format detection by magic bytes, and the pure parsers for BMP, WAV, OBJ meshes and shader sources.
- **Not for:** codec libraries or ownership of loaded data: PNG is `meridian-asset-png`, compressed and streaming audio is `meridian-asset-symphonia`, identity is `meridian-resource-core`, storage `meridian-memory-core`.

## Usage

This sketch is not compiled as a doc test (it needs a device, a window or a build script).

```rust,ignore
use meridian_asset_core::{Decoder, ObjDecoder};

let mesh = ObjDecoder.decode(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n")?;
```

## Depends on / used by

- Depends on: [`meridian-foundation`](../meridian-foundation)
- Used by: [`meridian-asset-png`](../meridian-asset-png), [`meridian-asset-symphonia`](../meridian-asset-symphonia), [`meridian-graphics-driver`](../meridian-graphics-driver), [`meridian-rendering-app`](../meridian-rendering-app), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

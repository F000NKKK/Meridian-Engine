# meridian-asset-core

Loading and decoding of images, meshes, audio, shaders and binary formats. Does not manage ownership or lifetime of loaded data.

**Layer and context:** domain · assets

- **For:** Decoders for images (PNG, BMP), OBJ meshes, audio (WAV, compressed, streaming) and shader sources, returning plain CPU data.
- **Not for:** ownership or lifetime of loaded data: identity is `meridian-resource-core`, storage `meridian-memory-core`.

## Usage

This sketch is not compiled as a doc test (it needs a device, a window or a build script).

```rust,ignore
use meridian_asset_core::{Decoder, ObjDecoder};

let mesh = ObjDecoder.decode(b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n")?;
```

## Depends on / used by

- Depends on: [`meridian-foundation`](../meridian-foundation)
- Used by: [`meridian-graphics-core`](../meridian-graphics-core), [`meridian-graphics-driver`](../meridian-graphics-driver), [`meridian-rendering-app`](../meridian-rendering-app), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

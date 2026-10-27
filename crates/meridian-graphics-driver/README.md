# meridian-graphics-driver

The `wgpu` rendering adapter: device, surface, pipelines, and the recording of scenes, sprites and bloom into GPU passes.

**Layer and context:** infrastructure (adapter) · rendering

- **For:** implementing `graphics-core`'s `FrameRecorder` (`GpuFrameRecorder`) and `TextureStore` (`GpuTextureStore`); the graphics device (headless or windowed), surfaces, render pipelines, scene/sprite/bloom recording.
- **Not for:** scenes, materials, meshes or the render graph: those are `meridian-graphics-core`.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-graphics-driver); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-asset-core`](../meridian-asset-core), [`meridian-foundation`](../meridian-foundation), [`meridian-gac-core`](../meridian-gac-core), [`meridian-gpu-driver`](../meridian-gpu-driver), [`meridian-graphics-core`](../meridian-graphics-core), [`meridian-memory-core`](../meridian-memory-core), [`meridian-platform-core`](../meridian-platform-core)
- Used by: [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

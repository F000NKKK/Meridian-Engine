# meridian-rendering-wgpu

The `wgpu` rendering adapter: device, surface, pipelines, and the recording of scenes, sprites and bloom into GPU passes.

**Layer and context:** infrastructure (adapter) · rendering

- **For:** implementing `rendering-domain`'s `FrameRecorder` (`GpuFrameRecorder`) and `TextureStore` (`GpuTextureStore`); the graphics device (headless or windowed), surfaces, render pipelines, scene/sprite/bloom recording.
- **Not for:** scenes, materials, meshes or the render graph: those are `meridian-rendering-domain`.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-rendering-wgpu); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-asset-domain`](../meridian-asset-domain), [`meridian-diagnostics`](../meridian-diagnostics), [`meridian-gac`](../meridian-gac), [`meridian-gpu-wgpu`](../meridian-gpu-wgpu), [`meridian-rendering-domain`](../meridian-rendering-domain), [`meridian-memory`](../meridian-memory), [`meridian-platform-winit`](../meridian-platform-winit), [`meridian-rendering-app`](../meridian-rendering-app)
- Used by: [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

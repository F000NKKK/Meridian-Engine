# meridian-gpu-wgpu

Shared low-level wgpu device/buffer/shader/compute-pipeline mechanics underneath rendering-wgpu and compute-wgpu. Knows nothing about scenes, materials, or compute-dispatch scheduling policy.

**Layer and context:** infrastructure (adapter) · compute

- **For:** The shared `wgpu` device, buffers, textures, shaders, bind groups and compute pipelines.
- **Not for:** scenes, materials or dispatch policy: see `meridian-rendering-wgpu` and `meridian-compute-wgpu`.

## Usage

This sketch is not compiled as a doc test (it needs a device, a window or a build script).

```rust,ignore
use meridian_gpu_wgpu::Device;

// Needs a GPU (or a software adapter); tests that do are behind `hw-tests`.
let device = Device::new().await?;
```

## Features

| Feature | Enables |
|---|---|
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-diagnostics`](../meridian-diagnostics), [`meridian-platform-winit`](../meridian-platform-winit)
- Used by: [`meridian-compute-wgpu`](../meridian-compute-wgpu), [`meridian-gac-wgpu`](../meridian-gac-wgpu), [`meridian-rendering-wgpu`](../meridian-rendering-wgpu), [`meridian-physics-wgpu`](../meridian-physics-wgpu)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

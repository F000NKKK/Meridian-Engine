# meridian-gpu-driver

Shared low-level wgpu device/buffer/shader/compute-pipeline mechanics underneath graphics-driver and compute-driver. Knows nothing about scenes, materials, or compute-dispatch scheduling policy.

**Layer and context:** infrastructure (adapter) · compute

- **For:** The shared `wgpu` device, buffers, textures, shaders, bind groups and compute pipelines.
- **Not for:** scenes, materials or dispatch policy: see `meridian-graphics-driver` and `meridian-compute-driver`.

## Usage

This sketch is not compiled as a doc test (it needs a device, a window or a build script).

```rust,ignore
use meridian_gpu_driver::Device;

// Needs a GPU (or a software adapter); tests that do are behind `hw-tests`.
let device = Device::new().await?;
```

## Features

| Feature | Enables |
|---|---|
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-foundation`](../meridian-foundation), [`meridian-platform-core`](../meridian-platform-core)
- Used by: [`meridian-compute-driver`](../meridian-compute-driver), [`meridian-gac-compute`](../meridian-gac-compute), [`meridian-graphics-driver`](../meridian-graphics-driver), [`meridian-physics-compute`](../meridian-physics-compute)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

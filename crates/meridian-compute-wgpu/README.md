# meridian-compute-wgpu

Low-level compute dispatch abstraction (CPU SIMD backends, GPU compute queues/buffers). Knows nothing about physics, animation or rendering.

**Layer and context:** infrastructure (adapter) · compute

- **For:** CPU-SIMD and GPU-compute dispatch mechanics, with a capability report.
- **For:** implementing `compute-app`'s `ComputeDevice` (`CpuComputeDevice`, OS threads) and `GpuBackend` (`GpuComputeDevice`, wgpu) ports; `cpu_context`/`gpu_context` build a `ComputeContext` from them.
- **Not for:** physics, animation or rendering algorithms; dispatch scheduling policy is `meridian-compute-app`.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-compute-wgpu); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-compute-app`](../meridian-compute-app), [`meridian-gpu-wgpu`](../meridian-gpu-wgpu), [`meridian-platform-winit`](../meridian-platform-winit)
- Used by: [`meridian-gac-wgpu`](../meridian-gac-wgpu), [`meridian-physics-wgpu`](../meridian-physics-wgpu), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

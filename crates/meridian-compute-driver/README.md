# meridian-compute-driver

Low-level compute dispatch abstraction (CPU SIMD backends, GPU compute queues/buffers). Knows nothing about physics, animation or rendering.

**Layer and context:** infrastructure (adapter) · compute

- **For:** CPU-SIMD and GPU-compute dispatch mechanics, with a capability report.
- **For:** implementing `compute-runtime`'s `ComputeDevice` (`CpuComputeDevice`, OS threads) and `GpuBackend` (`GpuComputeDevice`, wgpu) ports; `cpu_context`/`gpu_context` build a `ComputeContext` from them.
- **Not for:** physics, animation or rendering algorithms; dispatch scheduling policy is `meridian-compute-runtime`.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-compute-driver); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-compute-runtime`](../meridian-compute-runtime), [`meridian-gpu-driver`](../meridian-gpu-driver), [`meridian-platform-core`](../meridian-platform-core)
- Used by: [`meridian-gac-compute`](../meridian-gac-compute), [`meridian-physics-compute`](../meridian-physics-compute), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

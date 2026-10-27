# meridian-compute-app

Compute dispatch runtime: device/context, buffers, and the `ComputeKernel` interface domain crates (gac-wgpu, future particle-compute/physics-compute/...) implement against. Owns no algorithms of its own.

**Layer and context:** application · compute

- **For:** the ports `ComputeDevice` and `GpuBackend` (with fakes and contract suites under the `testing` feature), `ComputeContext`, the `ComputeKernel` and `HybridKernel` interfaces, `DispatchSize` and CPU/GPU `BackendSplit`.
- **Not for:** algorithms: kernels are implemented by adapter crates such as `meridian-gac-wgpu` and `meridian-physics-wgpu`.

## Usage

```rust
use meridian_compute_app::DispatchSize;

let size = DispatchSize::linear(1000);
assert_eq!(size.total(), 1000);
```

## Features

| Feature | Enables |
|---|---|
| `testing` | `testing::SequentialDevice`, `testing::FakeGpu` and the `run_device_contract`/`run_gpu_contract` suites |
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: no workspace crate
- Used by: [`meridian-compute-wgpu`](../meridian-compute-wgpu), [`meridian-runtime-app`](../meridian-runtime-app), [`meridian-gac-wgpu`](../meridian-gac-wgpu), [`meridian-physics-wgpu`](../meridian-physics-wgpu), [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

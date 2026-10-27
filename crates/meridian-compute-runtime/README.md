# meridian-compute-runtime

Compute dispatch runtime: device/context, buffers, and the `ComputeKernel` interface domain crates (gac-compute, future particle-compute/physics-compute/...) implement against. Owns no algorithms of its own.

**Layer and context:** application · compute

- **For:** `ComputeContext`, the `ComputeKernel` and `HybridKernel` interfaces, `DispatchSize` and CPU/GPU `BackendSplit`.
- **Not for:** algorithms: kernels are implemented by adapter crates such as `meridian-gac-compute` and `meridian-physics-compute`.

## Usage

```rust
use meridian_compute_runtime::DispatchSize;

let size = DispatchSize::linear(1000);
assert_eq!(size.total(), 1000);
```

## Depends on / used by

- Depends on: [`meridian-compute-driver`](../meridian-compute-driver), [`meridian-memory-core`](../meridian-memory-core), [`meridian-task-core`](../meridian-task-core)
- Used by: [`meridian-engine-core`](../meridian-engine-core), [`meridian-gac-compute`](../meridian-gac-compute), [`meridian-graphics-core`](../meridian-graphics-core), [`meridian-physics-compute`](../meridian-physics-compute), [`meridian-physics-core`](../meridian-physics-core)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

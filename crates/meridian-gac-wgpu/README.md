# meridian-gac-wgpu

Batch execution kernels for GAC transforms (Motor3) on CPU-SIMD / GPU compute. The adapter between the pure math in gac and the dispatch runtime in compute-app — neither depends on the other.

**Layer and context:** infrastructure (adapter) · geometry

- **For:** Batch `Motor3` transform and compose kernels for CPU-SIMD and GPU compute, float and `Fixed` (WGSL).
- **Not for:** pure math (`meridian-gac`) or dispatch mechanics (`meridian-compute-app`): it is the adapter between them.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-gac-wgpu); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-compute-wgpu`](../meridian-compute-wgpu), [`meridian-compute-app`](../meridian-compute-app), [`meridian-gac`](../meridian-gac), [`meridian-gpu-wgpu`](../meridian-gpu-wgpu), [`meridian-numeric`](../meridian-numeric)
- Used by: [`meridian-physics-wgpu`](../meridian-physics-wgpu)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

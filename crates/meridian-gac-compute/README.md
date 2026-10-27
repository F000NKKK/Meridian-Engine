# meridian-gac-compute

Batch execution kernels for GAC transforms (Motor3) on CPU-SIMD / GPU compute. The adapter between the pure math in gac-core and the dispatch runtime in compute-runtime — neither depends on the other.

**Layer and context:** infrastructure (adapter) · geometry

- **For:** Batch `Motor3` transform and compose kernels for CPU-SIMD and GPU compute, float and `Fixed` (WGSL).
- **Not for:** pure math (`meridian-gac-core`) or dispatch mechanics (`meridian-compute-runtime`): it is the adapter between them.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-gac-compute); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-compute-driver`](../meridian-compute-driver), [`meridian-compute-runtime`](../meridian-compute-runtime), [`meridian-gac-core`](../meridian-gac-core), [`meridian-gpu-driver`](../meridian-gpu-driver), [`meridian-numeric-core`](../meridian-numeric-core)
- Used by: [`meridian-physics-compute`](../meridian-physics-compute)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

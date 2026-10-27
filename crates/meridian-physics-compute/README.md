# meridian-physics-compute

Compute adapter for physics-core: WGSL soft-body (mass-spring) GPU kernels (float and Fixed flavors), plus compute-runtime-batched rigid-body Integrator and NarrowPhase::test_pair kernels (generic over GaFlavor, CPU dispatch today). The adapter between physics-core's domain model and compute-runtime's dispatch interface — mirrors meridian-gac-compute's role for GAC batch ops (see docs/dependency-rules.md rule 11).

**Layer and context:** infrastructure (adapter) · physics

- **For:** WGSL soft-body kernels and compute-runtime-batched rigid-body `Integrator` and `NarrowPhase::test_pair` kernels, generic over `GaFlavor`.
- **Not for:** the physics domain model: that is `meridian-physics-core`.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-physics-compute); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-compute-driver`](../meridian-compute-driver), [`meridian-compute-runtime`](../meridian-compute-runtime), [`meridian-foundation`](../meridian-foundation), [`meridian-gac-compute`](../meridian-gac-compute), [`meridian-gac-core`](../meridian-gac-core), [`meridian-gpu-driver`](../meridian-gpu-driver), [`meridian-numeric-core`](../meridian-numeric-core), [`meridian-physics-app`](../meridian-physics-app), [`meridian-physics-core`](../meridian-physics-core)
- Used by: [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

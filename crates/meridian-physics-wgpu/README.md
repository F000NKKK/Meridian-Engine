# meridian-physics-wgpu

Compute adapter for physics-domain: WGSL soft-body (mass-spring) GPU kernels (float and Fixed flavors), plus compute-runtime-batched rigid-body Integrator and NarrowPhase::test_pair kernels (generic over GaFlavor, CPU dispatch today). The adapter between physics-domain's domain model and compute-app's dispatch interface — mirrors meridian-gac-wgpu's role for GAC batch ops (see docs/dependency-rules.md rule 11).

**Layer and context:** infrastructure (adapter) · physics

- **For:** WGSL soft-body kernels and compute-runtime-batched rigid-body `Integrator` and `NarrowPhase::test_pair` kernels, generic over `GaFlavor`.
- **Not for:** the physics domain model: that is `meridian-physics-domain`.

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-physics-wgpu); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Features

| Feature | Enables |
|---|---|
| `hw-tests` | Tests that need a real GPU or audio device |

## Depends on / used by

- Depends on: [`meridian-compute-wgpu`](../meridian-compute-wgpu), [`meridian-compute-app`](../meridian-compute-app), [`meridian-diagnostics`](../meridian-diagnostics), [`meridian-gac-wgpu`](../meridian-gac-wgpu), [`meridian-gac`](../meridian-gac), [`meridian-gpu-wgpu`](../meridian-gpu-wgpu), [`meridian-numeric`](../meridian-numeric), [`meridian-physics-app`](../meridian-physics-app), [`meridian-physics-domain`](../meridian-physics-domain)
- Used by: [`meridian-sdk`](../meridian-sdk)

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

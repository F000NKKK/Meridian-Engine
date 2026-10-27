# meridian-physics-driver

Execution backend for physics: memory backend, SIMD/GPU dispatch and synchronization. Owns no collision algorithms, BVH, or broad-phase structures — those belong to physics-core.

**Layer and context:** infrastructure (adapter) · physics

- **For:** `PhysicsBackend`: reports CPU capability so physics can pick batch-size cutoffs. No GPU backend yet.
- **Not for:** collision algorithms, BVHs or broad-phase structures: those belong to `meridian-physics-core`. It is slated for deletion in wave 5 (ADR 019).

## Usage

The entry points are documented on [docs.rs](https://docs.rs/meridian-physics-driver); see `src/lib.rs` for the module overview and `docs/` in the repository for the design.

## Depends on / used by

- Depends on: [`meridian-platform-core`](../meridian-platform-core)
- Used by: no workspace crate

## License

MPL-2.0, see the repository [LICENSE](../../LICENSE).

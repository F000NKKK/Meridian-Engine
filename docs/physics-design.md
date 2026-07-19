# Physics design — `meridian-physics-driver` + `meridian-physics-core`

## The split

`physics-driver` is execution only: memory backend, SIMD/GPU dispatch,
synchronization. It owns **no** collision algorithms, no BVH, no
broad-phase structure — those are domain logic, not execution backend, and
belong to `physics-core`. This is a narrower scope than the
`graphics-driver`/`audio-driver` pattern might suggest at first glance:
BVH/spatial-hashing are how physics *reasons about space*, not how it
*executes work*, so they stay in `physics-core` even though they're
"low-level" in the algorithmic sense. See
[ADR 005](adr/005-driver-core-separation.md) and
[dependency-rules.md](dependency-rules.md) rule 2.

`physics-core` owns the actual simulation, including its own broad-phase.
Real, tested (not stub) as of this writing:

```text
Geometry              Sphere only (ColliderShape) — box/capsule/mesh later
Broad Phase           Naive O(n²) AABB sweep — spatial hash/BVH once profiling calls for it
Narrow Phase          Sphere-sphere exact test — GJK/EPA/SAT once more shapes exist
Constraint Solver     Impulse-based + positional correction against sinking
Integration           Semi-implicit Euler
```

`Aabb` (used by `BroadPhase`) is `gac-core`'s primitive, not a physics-local
type — see docs/gac-design.md. `ColliderShape::Sphere` currently stores
only a `radius`; its center comes from `RigidBody::frame`, so it isn't
`gac-core::Sphere` itself (which pairs a radius with its own `center`).
Adding a second collider shape (box, capsule) is the natural point to
revisit whether narrow-phase should route through `gac-core`'s generic
`Shape`/`ConvexVolume` machinery instead of the current analytic
sphere-sphere formula — not done now since there's only one shape to test
against itself.

## `RigidBody` uses the GAC frame, not a bespoke transform

```rust
struct RigidBody {
    frame: Motor3,   // from meridian-gac-core, shared with every other subsystem
    velocity: ...,
    mass: ...,
}
```

There is no physics-specific position/rotation pair to keep in sync with the
rendering transform — both read the same `Motor3`.

## Collider/mesh handles

Collision shapes referencing loaded mesh data go through
`meridian-resource-core`'s handle types, not a physics-specific handle —
see [memory-model.md](memory-model.md) and
[ADR 006](adr/006-resource-core-separation.md).

## Compute

Broad-phase and constraint solving are natural candidates for SIMD/GPU
parallelism at scale. `physics-core` reaches that through
`meridian-compute-runtime`, not by depending on `compute-driver` directly or
building its own scheduler — see
[dependency-rules.md](dependency-rules.md) rule 5. Not wired in yet: the
current `BroadPhase`/`NarrowPhase`/`ConstraintSolver`/`Integrator` are
correct sequential CPU implementations, called once per pair/body.
Batching them through `compute-runtime` (the same way
`gac-compute::MotorTransformKernel` batches `Motor3` composition) is
additive later — the same algorithm, called per-pair via a
`ComputeKernel` instead of a loop — not a rewrite.

`physics-driver`'s `PhysicsBackend` reports real CPU thread count (via
`platform-core::DeviceCapabilities`, the same shared shape
`compute-driver::ComputeCapabilities` uses) and `PhysicsSync` is a real
monotonic generation counter consumers can poll to know physics has
advanced — both implemented, neither wired into `physics-core`'s pipeline
yet.

## Determinism

A deterministic simulation mode (fixed-point or carefully-ordered
floating-point accumulation) is a design goal for reproducible replays and
networked simulation, but is not yet implemented — tracked in
[roadmap.md](roadmap.md).

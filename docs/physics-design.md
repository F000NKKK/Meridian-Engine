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

`physics-core` owns the actual simulation, including its own broad-phase:

```text
Geometry
Broad Phase          BVH, spatial hashing — owned here, not in physics-driver
Narrow Phase          GJK, EPA, SAT
Constraint Solver     impulse-based
Integration
```

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
`meridian-compute-core`, not by depending on `compute-driver` directly or
building its own scheduler — see
[dependency-rules.md](dependency-rules.md) rule 5.

## Determinism

A deterministic simulation mode (fixed-point or carefully-ordered
floating-point accumulation) is a design goal for reproducible replays and
networked simulation, but is not yet implemented — tracked in
[roadmap.md](roadmap.md).

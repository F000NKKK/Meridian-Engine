# Physics design — `meridian-physics-driver` + `meridian-physics-core`

## The split

`physics-driver` holds low-level, domain-agnostic spatial primitives: BVH
construction, spatial hashing, broad-phase acceleration structures. It has
no concept of a rigid body, a constraint, or a scene — the same separation
`graphics-driver`/`graphics-core` follow (see
[ADR 005](adr/005-driver-core-separation.md)).

`physics-core` builds the actual simulation on top:

```text
Geometry
Collision            GJK, EPA, SAT
Broad Phase           (via physics-driver)
Narrow Phase
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

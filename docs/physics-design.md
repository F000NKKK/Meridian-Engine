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

`physics-core`'s whole engine — `RigidBody`, `ColliderShape`, `Contact`,
`BroadPhase`, `NarrowPhase`, `ConstraintSolver`, `Integrator` — is written
**once**, in `src/generic.rs`, generic over
`meridian_gac_core::generic::GaFlavor`. Unlike `gac-core::Motor3`/`Vec3`
(concretely duplicated between `float_ga`/`fixed_ga` because
`meridian-gac-compute` dispatches them to a GPU with no real `i64`),
nothing in this engine has a GPU-dispatch constraint of its own — broad
phase, narrow phase, constraint solving and integration are the same
sequence of operations regardless of scalar flavor. `src/float.rs` and
`src/fixed.rs` are thin `FloatFlavor`/`FixedFlavor` type aliases over that
one engine, not a second copy of it (see CLAUDE.md's "Float/Fixed
branching" rule and [ADR 008](adr/008-fixed-point-determinism.md)).
`float`'s aliases are re-exported at the crate root, so
`meridian_physics_core::RigidBody`/etc. resolve to the default `f32` path
unchanged; `fixed`'s aliases are the deterministic path, see
"Determinism" below.

`physics-core` owns the actual simulation, including its own broad-phase.
Real, tested (not stub) as of this writing:

```text
Geometry              Sphere, Cuboid (ColliderShape) — capsule/mesh later
Broad Phase           Naive O(n²) AABB sweep — spatial hash/BVH once profiling calls for it
Narrow Phase          Sphere-sphere, sphere-cuboid, cuboid-cuboid (SAT) exact tests
Constraint Solver     Impulse-based + positional correction against sinking,
                       resolve split into resolve_velocity/
                       apply_positional_correction, restitution velocity
                       threshold for resting contacts
Integration           Semi-implicit Euler
```

`Aabb` (used by `BroadPhase`) is `gac-core`'s primitive, not a physics-local
type — see docs/gac-design.md. `ColliderShape::Sphere` still stores only a
`radius` (its center comes from `RigidBody::frame`, so it isn't
`gac-core::Sphere` itself, which pairs a radius with its own `center`).
`ColliderShape::Cuboid` stores only `half_extents`; `RigidBody::as_obb`
builds the world-space `gac-core::Obb` on demand from the body's own
`frame` — no second orientation to keep in sync (see docs/gac-design.md's
note on `Obb`'s `frame: Motor3` field).

Narrow phase does *not* go through `gac-core`'s generic `Shape`/
`ConvexVolume` machinery for collision detection itself (that machinery
answers "is X inside this convex region", a boolean/containment question —
narrow phase needs a contact point, normal *and* penetration depth, which
is a different, harder problem). What it does reuse from `Shape`:
`RigidBody::aabb_of`'s broad-phase bound (`Obb::support` along each world
axis) and cuboid-cuboid's contact point (`Obb::support` along the chosen
SAT axis from each box) — both genuinely the same "any convex shape, one
interface" idea, just not the boolean containment test. Sphere-cuboid uses
a closest-point-on-box formula; cuboid-cuboid uses the separating axis
theorem (SAT: 6 face-normal axes + 9 edge-cross-product axes for a box
pair) — the two techniques `roadmap.md` already anticipated for this step.
`NarrowPhase::test_pair` collapses both to a single contact point per
pair — the simple exact-overlap query, and the same simplification
`ConstraintSolver`'s doc comment discloses for its angular response.
`NarrowPhase::generate_contacts` (what `ConstraintSolver::resolve`
should actually iterate over — see "Resting-contact stabilization"
below) is the manifold-aware sibling: a cuboid-cuboid pair expands into
up to 4 face-manifold contact points (`face_manifold`, one per corner of
the incident box's near face that falls inside the reference face's
bounds), each carrying an equal share of the pair's total penetration
and `suppress_angular_response: true` (per-corner torque isn't a
coupled solve — see that field's own doc comment for why). Sphere pairs
and edge/corner cuboid contacts still collapse to `test_pair`'s single
point inside `generate_contacts` too — only a face-face box manifold
actually produces more than one. `RigidBody::moment_of_inertia` for
`Cuboid` is the average of
the box's three true principal moments, not the full anisotropic tensor —
disclosed on that method's own doc comment, needed because
`ConstraintSolver` only has a single scalar `inverse_inertia` to work
with, not per-axis.

## Resting-contact stabilization

`ConstraintSolver::resolve` originally applied both the velocity impulse
and the positional (Baumgarte-style) correction in one call. Multi-point
manifolds relax over several passes per tick, so calling `resolve`
per-pass over-applied the positional push each time — visible as jitter
and floor-clipping on settled boxes/pyramids. `resolve` is now split:
`resolve_velocity` is safe to call once per relaxation pass,
`apply_positional_correction` must be called exactly once per tick, after
the relaxation loop — callers control the pass count without compounding
position correction.

Separately, `ConstraintSolver` has a `restitution_velocity_threshold`
field (default 0.5 world-units/second, set via
`with_restitution_velocity_threshold`): below that closing speed,
restitution is suppressed. Without it, gravity re-applied every tick
against a nonzero restitution produces a never-ending,
shrinking-but-nonzero bounce on resting contacts — negligible for a
single sphere, visible as continuous up/down micro-jitter on multi-point
manifolds. Real impacts (well above the threshold) are unaffected.
`examples/physic_figures` additionally sets its solver restitution to
`0.0`, making landings fully inelastic on top of the threshold.

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
[dependency-rules.md](dependency-rules.md) rule 5. `BroadPhase`/
`NarrowPhase`/`ConstraintSolver`/`Integrator` are still correct
sequential CPU implementations in `physics-core` itself, called once per
pair/body — batching is an adapter-crate concern (rule 11), not
something `physics-core` does itself. `Integrator` is batched now:
`meridian-physics-compute::rigid_body::RigidBodyIntegratorKernel` runs
`Integrator::step` across many bodies through one `ComputeKernel`
dispatch (the same shape `gac-compute::MotorTransformKernel` batches
`Motor3` composition with), generic over `GaFlavor` and CPU-dispatched
via `ComputeContext::parallel_for` — `Integrator::step` has no
GPU-dispatch constraint of its own, so there's no WGSL kernel to write,
unlike this crate's soft-body kernels. `NarrowPhase::test_pair` is
batched too, the same way:
`meridian-physics-compute::narrow_phase::NarrowPhaseTestPairKernel`
tests many candidate pairs for exact overlap through one dispatch,
mirroring `gac-compute::MotorComposeKernel`'s independent-pairs shape
(`test_pair` is one pair in, one `Option<Contact<F>>` out — a fixed 1:1
shape, unlike `generate_contacts`' variable-size manifold output, see
below). `BroadPhase::find_candidate_pairs` and
`NarrowPhase::generate_contacts`/`ConstraintSolver` aren't batched yet:
`find_candidate_pairs`' AABB sweep isn't an independent per-item
computation the way `Integrator::step`/`test_pair` are, and
`generate_contacts`/the solver carry more per-pair/per-contact state
(box-box pairs expand to a *variable* number of manifold points,
accumulated impulses) than a fixed-size-per-item kernel shape covers —
real follow-up (a per-pair count prefix-sum into a flattened output
buffer, the standard GPU technique for variable-output-per-thread work,
for `generate_contacts`), not done here.

`physics-driver`'s `PhysicsBackend` reports real CPU thread count (via
`platform-core::DeviceCapabilities`, the same shared shape
`compute-driver::ComputeCapabilities` uses) and `PhysicsSync` is a real
monotonic generation counter consumers can poll to know physics has
advanced — both implemented, neither wired into `physics-core`'s pipeline
yet.

## Determinism

Real, via `physics-core::fixed` — see
[ADR 008](adr/008-fixed-point-determinism.md) for the full decision.
`fixed::RigidBody`/`fixed::Integrator`/`fixed::ConstraintSolver`/
`fixed::BroadPhase`/`fixed::NarrowPhase` are thin `FixedFlavor` aliases
over the exact same generic engine (`src/generic.rs`) `float::RigidBody`/
etc. alias with `FloatFlavor` — built on `gac-core::fixed_ga` (`Fixed`,
Q16.16) instead of `float_ga` (`f32`) via the `GaFlavor` trait, a
genuinely separate, opt-in pipeline, not a mode flag on the default
types. `fixed::BroadPhase` builds its `Aabb<FixedFlavor>`s from
`gac-core::generic` the same way `float::BroadPhase` builds
`Aabb<FloatFlavor>`s — geometry is `gac-core`'s job regardless of scalar
flavor, not reimplemented per consumer (see docs/gac-design.md). Because
the engine is generic rather than hand-duplicated, sphere-sphere,
sphere-cuboid *and* cuboid-cuboid (SAT) narrow phase all work for
`fixed::RigidBody` for free — there is no "sphere only" scope limit to
track as follow-up, unlike the earlier hand-duplicated
`physics-core::deterministic` module this replaced.
`FixedMotor3::to_float_lossy` (called on `fixed::RigidBody::frame`)
converts a pose to `gac-core::Motor3` for rendering/ECS/audio handoff — a
named, deliberate precision-changing cast (see docs/gac-design.md's
"Cross-flavor interop" section), not a `From`/`Into` that would make the
cast look free. Proven with an actual bit-exact reproducibility test (the
same scenario run twice produces identical `Fixed` bit patterns, not just
approximately equal floats) — `cargo test -p meridian-physics-core`.

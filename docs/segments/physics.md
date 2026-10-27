# Segment: physics (`team/physics`)

Ownership note and backlog for the physics segment. Design rationale lives
in [physics-design.md](../physics-design.md); implementation status for the
whole engine lives in [roadmap.md](../roadmap.md). This file only says who
owns what and what the segment plans to do next.

## Owned crates

| Crate | Tier | Role |
|---|---|---|
| `meridian-physics-driver` | 2 | Execution backend only: `PhysicsBackend` (CPU/GPU capabilities), `PhysicsSync` (step generation counter). No collision algorithms. |
| `meridian-physics-core` | 6 | The simulation: `RigidBody`, `ColliderShape`, `Contact`, `BroadPhase`, `NarrowPhase`, `ConstraintSolver`, `Integrator`, written once in `generic.rs` over `GaFlavor`; `float.rs`/`fixed.rs` are thin aliases. `soft_body/` is the separate mass-spring domain. |
| `meridian-physics-compute` | 7 | Adapter to `compute-runtime`: batched kernels for all five rigid-body pipeline steps (CPU `parallel_for`), WGSL soft-body kernels, WGSL broad-phase overlap kernel. |

Shared with other segments (physics reviews changes, the other segment owns
the crate):

- `meridian-engine-core` (`team/core`): `PhysicsSubsystem`,
  `PhysicsStepStage`, `PhysicsDispatchMode`, fixed-timestep `tick_fixed`.
- `meridian-gac-core` / `meridian-gac-compute` (`team/core`): `Motor3`,
  bivectors, `Aabb`/`Obb`/`Shape`. Geometry goes there, never into
  `physics-core`.
- `meridian-gpu-driver` (`team/rendering`): device used by the WGSL kernels.
- `examples/physic_figures`: physics showcase example.

Skills that apply: `meridian-subsystem-core-keeper`,
`meridian-gac-math-keeper`, `meridian-compute-driver-keeper`,
`meridian-crate-boundary-keeper`.

## Current state (2026-10-27)

- `cargo test -p meridian-physics-core -p meridian-physics-driver
  -p meridian-physics-compute`: 55 tests, all green.
- Shapes: sphere and cuboid. Broad phase: naive O(n²) AABB sweep. Narrow
  phase: sphere-sphere, sphere-cuboid, cuboid-cuboid SAT with up to 4-point
  face manifolds. Solver: sequential impulses with split velocity/position
  passes and a restitution threshold. Integrator: semi-implicit Euler with
  the bivector exponential map.
- Deterministic `fixed` (Q16.16) pipeline with a bit-exact reproducibility
  test.
- Batched dispatch is wired into `engine-core` through
  `PhysicsDispatchMode::Batched` and matches the sequential path bit for bit.

## Gaps and backlog

Ordered roughly by priority. Nothing here is started; each item waits for
a task.

### Hygiene (small, safe)

1. **Unused dependencies in `physics-core/Cargo.toml`.** `meridian-gac-compute`,
   `meridian-compute-runtime`, `meridian-physics-driver` and
   `meridian-ecs-core` are declared but not referenced in `src/`. They
   overstate the crate graph that `check-deps` validates. Remove them or
   use them deliberately.
2. **Stale crate docs.** `physics-core/src/lib.rs` still says no
   `compute-runtime` dispatch is wired in (it is, via `physics-compute`).
   `physics-driver/src/lib.rs` says the GPU backend waits for
   `graphics-driver`, but `meridian-gpu-driver` exists and the WGSL
   kernels already use it.
3. **`PhysicsSync` and `PhysicsBackend` are not used by the pipeline.**
   Either wire them in (batch-size cutoffs, consumer change detection) or
   document them as intentionally unused.
4. **Panic audit.** `unwrap`/`expect` calls in non-test code paths of
   `physics-compute` (for example mutex locks in the constraint-solver
   kernel) should be reviewed against the standards `team/standards` is
   writing in Craftbook.

### Features

5. **Spatial broad phase** (roadmap gap 3): uniform grid keyed to chunk
   size or a BVH, so cost scales with nearby bodies. Needed for voxel and
   factory scenes with thousands of static colliders. Add static/sleeping
   body support alongside it.
6. **More shapes**: capsule, convex hull, then triangle mesh and heightfield
   colliders referenced through `resource-core` handles.
7. **Full inertia tensor**: `moment_of_inertia` for cuboids is the average
   of the principal moments; the solver needs a per-axis inverse inertia.
8. **Coupled manifold solve**: face-manifold points use
   `suppress_angular_response`; a proper accumulated-impulse solver with
   warm starting would remove that simplification.
9. **Queries**: raycast, shape cast and overlap queries for gameplay.
10. **Joints and constraints** beyond contacts (hinge, ball, fixed).
11. **Friction per material**: the solver has one global coefficient (`with_friction`); split into static and dynamic coefficients per collider material.
12. **CCD** for fast bodies.

### GPU path

13. Port integrator, narrow phase and solver to WGSL with the same
    bit-for-bit oracle tests the broad-phase kernel has, then add
    `PhysicsDispatchMode::Gpu` only when all four stages have a GPU path
    (see roadmap gap 4).

### Standards alignment

14. Once Craftbook (`team/standards`) publishes its Rust rules, audit the
    three crates against them (error types, panics, naming, module layout,
    test layout) and record deviations here.

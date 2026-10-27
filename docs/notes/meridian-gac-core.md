# `meridian-gac-core` design notes

Rationale moved out of the module docs (Craftbook ch. 5 §5.2: a `//!` is at
most 30 lines). Each section belongs to the module named in its heading.

## `lib`

Everything else — geometric primitives (`Aabb`, `Sphere`, `Obb`,
`Cone`, `Plane`, `Shape`, `ConvexVolume`, `Projection`, `Frame`) —
lives in the `volume`, `projection`, `mesh` and `motor` modules, written **once**, generic over `GaFlavor`,
because none of it has the float/fixed `Motor3`-specific
GPU-dispatch constraint: an AABB overlap test or a projection matrix
derivation is the same sequence of operations regardless of which
scalar type it runs on, so duplicating it would just be maintenance
risk for no reason (see `CLAUDE.md`'s "Float/Fixed branching" rule).
each concept's `float.rs`/`fixed.rs` exposes thin type aliases
(`volume::float::Aabb = volume::Aabb<FloatFlavor>`,
`volume::fixed::FixedAabb = volume::Aabb<FixedFlavor>`, ...) so existing
call sites don't need to think about the generic parameter at all —
only code that wants to *be* generic over the flavor (like
`physics-core`'s engine) reaches into those modules directly.

See docs/gac-design.md and
[ADR 001](../adr/001-geometric-algebra-as-spatial-model.md).

## `flavor`

This is deterministic-simulation opt-in, not the default —
the float `Motor3` (`f32`) remains the everyday, GPU-dispatchable
pose type for everything else (rendering, ECS, audio, and
`physics-core`'s own default `float::RigidBody`). But it's *usable by
any crate* that needs CPU-deterministic geometry (`physics-core`'s
`fixed::RigidBody` today, potentially a deterministic `graphics-core`
CPU path or a large precise CPU/GPU-emulated simulation later) —
that reusability is exactly why the primitives below live here in
`gac-core`, not inside `physics-core`, the same reason the float
primitives aren't `physics-core`-local either. `physics-core`'s own
engine (`RigidBody`/`BroadPhase`/`NarrowPhase`/`ConstraintSolver`/
`Integrator`) goes a step further: it has no GPU-dispatch constraint
of its own at all, so it's written once, generic over
`crate::flavor::GaFlavor`, rather than duplicated like the float and fixed
algebra types are here.

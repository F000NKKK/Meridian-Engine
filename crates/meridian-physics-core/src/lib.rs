//! Rigid body dynamics built on the GAC: broad/narrow phase collision, constraint solving and integration.
//!
//! Real, tested physics pipeline: `BroadPhase` (naive O(n²) AABB sweep —
//! a spatial hash/BVH is a later optimization once profiling calls for
//! it, same policy as `task-core`'s scheduler), `NarrowPhase`
//! (sphere-sphere, sphere-cuboid, and cuboid-cuboid via SAT — the two
//! `ColliderShape` variants that exist so far), `ConstraintSolver`
//! (impulse-based, linear *and* angular, with positional correction
//! against sinking), and `Integrator` (semi-implicit Euler, using
//! `gac-core`'s bivector exponential map for rotation — not a naive "add
//! angle" or a separately-tracked quaternion). No GPU/SIMD dispatch
//! through `compute-runtime` wired in yet — these are correct sequential
//! CPU implementations first; batching them through `compute-runtime` is
//! additive later, not a rewrite (the same kernel logic, called per-pair
//! instead of once).
//!
//! GA is used where the physics actually calls for it, not decoratively:
//! angular velocity and torque are bivectors (they live in the Lie
//! algebra of rotations, so(3) — a bivector space, not a vector space),
//! and orientation is integrated via the bivector exponential map,
//! composed onto the body's motor — this is what keeps a spinning body's
//! orientation exactly on the unit-rotor manifold frame after frame, the
//! same reason `gac-core` uses motors for `Transform` at all instead of a
//! quaternion+vector pair (see
//! [ADR 001](../../../docs/adr/001-geometric-algebra-as-spatial-model.md)).
//! Linear velocity stays a plain vector — GA doesn't say vectors are
//! wrong, only that angular quantities specifically are bivectors.
//!
//! The whole engine — [`generic::RigidBody`], [`generic::ColliderShape`],
//! [`generic::Contact`], [`generic::BroadPhase`], [`generic::NarrowPhase`],
//! [`generic::ConstraintSolver`], [`generic::Integrator`] — is written
//! **once**, generic over `meridian_gac_core::generic::GaFlavor`: unlike
//! `gac-core::Motor3`/`Vec3` (concretely duplicated because
//! `meridian-gac-compute` dispatches them to a GPU with no real `i64`),
//! nothing in this engine has a GPU-dispatch constraint of its own, so
//! duplicating it per scalar flavor would just be maintenance risk for no
//! reason (see CLAUDE.md's "Float/Fixed branching" rule). [`float`] and
//! [`fixed`] are thin `FloatFlavor`/`FixedFlavor` aliases over the one
//! engine in [`generic`] — `float` is re-exported at the crate root (so
//! `meridian_physics_core::RigidBody`/etc. resolve to the default `f32`
//! path unchanged), `fixed` is an opt-in, bit-reproducible alternative
//! for lockstep networking/replay — see that module's doc comment.
//! Nothing about `float`'s types changes when `fixed` exists; it's a
//! parallel API, not a mode switch.
//!
//! [`soft_body`] is a separate domain, deformable bodies (mass-spring),
//! not glob-exported at this crate's root — see that module's own doc
//! comment for why it's a genuinely different problem from the rigid-body
//! engine above, and its own float/fixed split.

pub mod fixed;
pub mod float;
pub mod generic;
pub mod soft_body;

pub use float::*;

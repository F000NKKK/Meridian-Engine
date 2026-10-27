//! Generic rigid-body physics engine — [`RigidBody`], [`ColliderShape`],
//! [`Contact`], [`BroadPhase`], [`NarrowPhase`], [`ConstraintSolver`] and
//! [`Integrator`], all written once, generic over `F: GaFlavor`.
//!
//! Unlike `gac-core::Motor3`/`Vec3`/`Bivector3`/`Rotor` (concretely
//! duplicated between `float_ga` and `fixed_ga` because
//! `meridian-gac-compute` dispatches them to a GPU that has no real
//! `i64`), nothing in this engine has a GPU-dispatch constraint of its
//! own — broad phase, narrow phase, constraint solving and integration
//! are the same sequence of operations regardless of which scalar type
//! backs them. So unlike `gac-core`'s `float_ga`/`fixed_ga` split, this
//! crate's `float.rs`/`fixed.rs` are thin `FloatFlavor`/`FixedFlavor`
//! aliases over the one engine defined here — see CLAUDE.md's
//! "Float/Fixed branching" rule.
//!
//! Deliberately its own module, not defined directly at the crate root
//! next to `pub use float::*;` — a locally-defined item always shadows a
//! glob-imported same-named item in Rust, so `RigidBody<F>` living at
//! the crate root would silently shadow `float::RigidBody`'s re-export
//! and break every existing unparameterized `meridian_physics_core::RigidBody`
//! call site. See `gac-core::generic`'s doc comment for the same bug,
//! caught and fixed there first.

mod axes;
mod broad_phase;
mod collider_mesh_handle;
mod collider_shape;
mod constraint_solver;
mod contact;
mod cuboid_contact;
mod integrator;
mod narrow_phase;
mod rigid_body;
mod sphere_contact;

pub(crate) use axes::{axis_x, axis_y, axis_z};
pub use broad_phase::BroadPhase;
pub(crate) use broad_phase::aabb_of;
pub use collider_mesh_handle::{ColliderMeshHandle, ColliderMeshMarker};
pub use collider_shape::ColliderShape;
pub use constraint_solver::ConstraintSolver;
pub use contact::Contact;
pub(crate) use cuboid_contact::cuboid_vs_cuboid;
pub use integrator::Integrator;
pub use narrow_phase::NarrowPhase;
pub use rigid_body::RigidBody;
pub(crate) use sphere_contact::{sphere_vs_cuboid, sphere_vs_sphere};

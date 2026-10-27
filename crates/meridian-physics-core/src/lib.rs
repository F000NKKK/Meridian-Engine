//! Rigid body dynamics built on the GAC: broad/narrow phase collision, constraint solving and integration.
//!
//! Layer: domain; bounded context: physics (`craftbook.toml`,
//! docs/context-map.md). A performance kernel (Craftbook §7.8): its
//! data-oriented layout is provisional until a benchmark justifies it
//! (ADR 020 D8).
//!
//! Real, tested physics pipeline: `BroadPhase` (naive O(n²) AABB sweep —
//! a spatial hash/BVH is a later optimization once profiling calls for
//! it, same policy as `task-core`'s scheduler), `NarrowPhase`
//! (sphere-sphere, sphere-cuboid, and cuboid-cuboid via SAT — the two
//! `ColliderShape` variants that exist so far), `ConstraintSolver`
//! (impulse-based, linear *and* angular, with positional correction
//! against sinking), and `Integrator` (semi-implicit Euler, using
//! `gac-core`'s bivector exponential map for rotation — not a naive "add
//! angle" or a separately-tracked quaternion). These are
//! correct sequential CPU implementations; batched and GPU dispatch lives
//! in `meridian-physics-compute`, which calls the same per-pair logic
//! through `compute-runtime`.
//!
//! Design notes: docs/notes/meridian-physics-core.md (`lib`).
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]

pub mod collision;
pub mod dynamics;
pub mod fixed;
pub mod float;
pub mod soft_body;

#[cfg(test)]
mod tests;

pub use float::*;

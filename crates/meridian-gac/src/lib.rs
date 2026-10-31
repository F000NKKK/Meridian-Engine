//! Geometric Algebra Core — vectors, bivectors, multivectors, rotors and motors; the shared spatial math foundation for every other subsystem.
//!
//! Layer: shared kernel; bounded context: geometry (`craftbook.toml`,
//! docs/context-map.md). A performance kernel (Craftbook §7.8): its
//! data-oriented layout is provisional until a benchmark justifies it
//! (ADR 020 D8).
//!
//! Two flavors per concept, each concept in a directory of its own with a
//! `float` module (`f32`, the default — re-exported at the crate root, so
//! `meridian_gac::Vec3`/`Motor3`/`Aabb`/etc. resolve to it) and a
//! `fixed` module (`Fixed`, deterministic, re-exported with a `Fixed`
//! prefix — usable by *any* crate that needs CPU-deterministic geometry,
//! not just `physics-domain`). Only `Multivector`/`Vec3`/`Bivector3`/
//! `Rotor`/`Motor3` themselves are concretely duplicated between the two
//! (`meridian-gac-wgpu`'s GPU dispatch needs the `f32` ones, ADR 008:
//! `meridian-gac-wgpu` dispatches them, and `compute-app` has no
//! GPU backend to dispatch to at all yet — CPU vs. GPU is a dispatch
//! *setting* a caller picks once one exists, not a type-level
//! restriction this crate imposes; a future GPU backend could in
//! principle run `Fixed` kernels too, at the cost of `i64` emulation and
//! losing the bit-exact determinism guarantee to GPU execution-order
//! nondeterminism — a tradeoff for the caller to accept knowingly, not
//! something forbidden here).
//!
//! Design notes: docs/notes/meridian-gac.md (`lib`).
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
#![expect(
    clippy::module_name_repetitions,
    reason = "`Fixed*` types live in `fixed` modules because both flavors are re-exported side by side at the crate root, where the prefix is what tells them apart"
)]

pub mod bivector;
pub mod blade;
#[cfg(feature = "data")]
mod data;
pub mod flavor;
pub mod mesh;
pub mod motor;
pub mod multivector;
pub mod projection;
pub mod rotor;
pub mod scale;
pub mod vector;
pub mod volume;

#[cfg(test)]
mod tests;

pub use bivector::fixed::*;
pub use bivector::float::*;
pub use blade::Blade;
#[cfg(feature = "data")]
pub use data::MotorForm;
pub use flavor::fixed::FixedFlavor;
pub use flavor::float::FloatFlavor;
pub use flavor::{BivectorLike, GaFlavor, MotorLike, RotorLike, ScalarLike, VectorLike};
pub use mesh::Face;
pub use mesh::fixed::*;
pub use mesh::float::*;
pub use motor::fixed::*;
pub use motor::float::*;
pub use multivector::fixed::*;
pub use multivector::float::*;
pub use projection::float::*;
pub use rotor::fixed::*;
pub use rotor::float::*;
pub use scale::{Scale3, ScaleError};
pub use vector::fixed::*;
pub use vector::float::*;
pub use volume::Shape;
pub use volume::fixed::*;
pub use volume::float::*;

pub use rotor::RotationError;

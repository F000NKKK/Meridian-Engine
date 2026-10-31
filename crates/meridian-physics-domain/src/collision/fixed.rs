//! Fixed-point-flavor aliases of the collision engine, an opt-in bit-reproducible path.

use meridian_gac::FixedFlavor;

/// A fixed-point collider shape.
pub type ColliderShape = super::ColliderShape<FixedFlavor>;
/// Fixed-point broad-phase collision detection.
pub type BroadPhase = super::BroadPhase<FixedFlavor>;
/// A fixed-point contact manifold point.
pub type Contact = super::Contact<FixedFlavor>;
/// Fixed-point narrow-phase collision detection.
pub type NarrowPhase = super::NarrowPhase<FixedFlavor>;

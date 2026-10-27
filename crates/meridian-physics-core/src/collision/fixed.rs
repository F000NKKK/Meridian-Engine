//! Fixed-point-flavor aliases of the collision engine, an opt-in bit-reproducible path.

use meridian_gac_core::FixedFlavor;

pub type ColliderShape = super::ColliderShape<FixedFlavor>;
pub type BroadPhase = super::BroadPhase<FixedFlavor>;
pub type Contact = super::Contact<FixedFlavor>;
pub type NarrowPhase = super::NarrowPhase<FixedFlavor>;

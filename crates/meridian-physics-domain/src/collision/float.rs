//! Float-flavor (`f32`) aliases of the collision engine.

use meridian_gac::FloatFlavor;

pub type ColliderShape = super::ColliderShape<FloatFlavor>;
pub type BroadPhase = super::BroadPhase<FloatFlavor>;
pub type Contact = super::Contact<FloatFlavor>;
pub type NarrowPhase = super::NarrowPhase<FloatFlavor>;

//! Thin `FloatFlavor` (`f32`) aliases over `crate::generic`'s engine —
//! the default, everyday physics path. See `crate::generic`'s module doc
//! for why this is just aliases, not a second copy of the engine.

pub use meridian_gac_core::FloatFlavor;

pub type ColliderShape = crate::generic::ColliderShape<FloatFlavor>;
pub type RigidBody = crate::generic::RigidBody<FloatFlavor>;
pub type BroadPhase = crate::generic::BroadPhase<FloatFlavor>;
pub type Contact = crate::generic::Contact<FloatFlavor>;
pub type NarrowPhase = crate::generic::NarrowPhase<FloatFlavor>;
pub type ConstraintSolver = crate::generic::ConstraintSolver<FloatFlavor>;
pub type Integrator = crate::generic::Integrator<FloatFlavor>;

#[cfg(test)]
mod tests;

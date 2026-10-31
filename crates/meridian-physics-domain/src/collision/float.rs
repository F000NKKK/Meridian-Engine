//! Float-flavor (`f32`) aliases of the collision engine.

use meridian_gac::FloatFlavor;

/// A floating-point collider shape.
pub type ColliderShape = super::ColliderShape<FloatFlavor>;
/// Floating-point broad-phase collision detection.
pub type BroadPhase = super::BroadPhase<FloatFlavor>;
/// A floating-point contact manifold point.
pub type Contact = super::Contact<FloatFlavor>;
/// Floating-point narrow-phase collision detection.
pub type NarrowPhase = super::NarrowPhase<FloatFlavor>;

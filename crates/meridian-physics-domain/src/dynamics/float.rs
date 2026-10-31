//! Float-flavor (`f32`) aliases of the dynamics engine.

use meridian_gac::FloatFlavor;

/// A rigid body with floating-point motion.
pub type RigidBody = super::RigidBody<FloatFlavor>;
/// A floating-point contact constraint solver.
pub type ConstraintSolver = super::ConstraintSolver<FloatFlavor>;
/// A floating-point rigid-body integrator.
pub type Integrator = super::Integrator<FloatFlavor>;

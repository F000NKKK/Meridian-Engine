//! Float-flavor (`f32`) aliases of the dynamics engine.

use meridian_gac_core::FloatFlavor;

pub type RigidBody = super::RigidBody<FloatFlavor>;
pub type ConstraintSolver = super::ConstraintSolver<FloatFlavor>;
pub type Integrator = super::Integrator<FloatFlavor>;

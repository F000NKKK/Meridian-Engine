//! Fixed-point-flavor aliases of the dynamics engine, an opt-in bit-reproducible path.

use meridian_gac_core::FixedFlavor;

pub type RigidBody = super::RigidBody<FixedFlavor>;
pub type ConstraintSolver = super::ConstraintSolver<FixedFlavor>;
pub type Integrator = super::Integrator<FixedFlavor>;

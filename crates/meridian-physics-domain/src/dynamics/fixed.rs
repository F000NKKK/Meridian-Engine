//! Fixed-point-flavor aliases of the dynamics engine, an opt-in bit-reproducible path.

use meridian_gac::FixedFlavor;

/// A rigid body with deterministic fixed-point motion.
pub type RigidBody = super::RigidBody<FixedFlavor>;
/// A fixed-point contact constraint solver.
pub type ConstraintSolver = super::ConstraintSolver<FixedFlavor>;
/// A fixed-point rigid-body integrator.
pub type Integrator = super::Integrator<FixedFlavor>;

//! Rigid-body dynamics written once, generic over `F: GaFlavor`: [`RigidBody`],
//! [`ConstraintSolver`] and [`Integrator`]. `float` and `fixed` bind it to the
//! two flavors.

mod constraint_solver;
pub mod fixed;
pub mod float;
mod integrator;
mod rigid_body;

pub use constraint_solver::ConstraintSolver;
pub use integrator::Integrator;
pub use rigid_body::RigidBody;

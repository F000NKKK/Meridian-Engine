//! A deformable body: particles connected by springs.

use meridian_gac_core::GaFlavor;

use super::spring::Spring;

/// A deformable body: particles (point masses, no orientation — unlike
/// [`crate::dynamics::RigidBody`], a soft body's "shape" *is* its particle
/// positions, not a fixed collider transformed by one frame) connected by
/// [`Spring`]s. `inverse_masses[i] <= 0` pins particle `i` in place
/// (never moved by [`SoftBodyIntegrator::step`]) — the same "static via
/// zero/negative inverse mass" convention
/// [`crate::dynamics::RigidBody::inverse_mass`] uses.
#[derive(Debug, Clone)]
pub struct SoftBody<F: GaFlavor> {
    pub positions: Vec<F::Vector>,
    pub velocities: Vec<F::Vector>,
    pub inverse_masses: Vec<F::Scalar>,
    pub springs: Vec<Spring<F>>,
}

impl<F: GaFlavor> SoftBody<F> {
    #[must_use]
    pub fn particle_count(&self) -> usize {
        self.positions.len()
    }
}

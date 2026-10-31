//! A deformable body: particles connected by springs.

use meridian_gac::GaFlavor;

use super::spring::Spring;

/// A deformable body: particles (point masses, no orientation — unlike
/// [`crate::dynamics::RigidBody`], a soft body's "shape" *is* its particle
/// positions, not a fixed collider transformed by one frame) connected by
/// [`Spring`]s. `inverse_masses[i] <= 0` pins particle `i` in place
/// (never moved by [`SoftBodyIntegrator::step`](super::time_step::SoftBodyIntegrator::step)) — the same "static via
/// zero/negative inverse mass" convention
/// [`crate::dynamics::RigidBody::inverse_mass`] uses.
#[derive(Debug, Clone)]
pub struct SoftBody<F: GaFlavor> {
    /// Current world-space particle positions.
    pub positions: Vec<F::Vector>,
    /// Current particle velocities.
    pub velocities: Vec<F::Vector>,
    /// Inverse mass per particle; zero pins a particle.
    pub inverse_masses: Vec<F::Scalar>,
    /// Springs connecting particles by index.
    pub springs: Vec<Spring<F>>,
}

impl<F: GaFlavor> SoftBody<F> {
    /// Returns the number of particles in the body.
    #[must_use]
    pub fn particle_count(&self) -> usize {
        self.positions.len()
    }
}

//! Time stepping of a soft body: springs, gravity and ground collision.

use meridian_gac::volume::Plane;
use meridian_gac::{GaFlavor, ScalarLike, VectorLike};

use super::mass_spring::SoftBody;

fn axis_x<F: GaFlavor>() -> F::Vector {
    F::Vector::new(F::Scalar::ONE, F::Scalar::ZERO, F::Scalar::ZERO)
}

/// Advances every particle in a [`SoftBody`] by one timestep: spring
/// forces (structural + damping), gravity, semi-implicit Euler
/// integration, then a ground-plane collision pass. See the module doc
/// for why this doesn't reuse the rigid-body engine's collision types.
#[derive(Debug, Clone, Copy)]
pub struct SoftBodyIntegrator<F: GaFlavor> {
    /// Constant acceleration applied to every unpinned particle.
    pub gravity: F::Vector,
    /// Ground half-space used for particle collisions.
    pub ground: Plane<F>,
    /// `0` = fully inelastic (a particle stops dead at the ground), `1`
    /// = fully elastic (no speed lost bouncing off the ground) — same
    /// convention as [`crate::dynamics::ConstraintSolver::restitution`].
    pub restitution: F::Scalar,
}

impl<F: GaFlavor> SoftBodyIntegrator<F> {
    /// Creates an integrator with gravity, a ground plane and restitution.
    pub fn new(gravity: F::Vector, ground: Plane<F>, restitution: F::Scalar) -> Self {
        Self {
            gravity,
            ground,
            restitution,
        }
    }

    /// Advances all particles by `dt` seconds.
    pub fn step(&self, body: &mut SoftBody<F>, dt: F::Scalar) {
        let forces = spring_forces(body);
        for (i, &force) in forces.iter().enumerate() {
            let inverse_mass = body.inverse_masses[i];
            if inverse_mass <= F::Scalar::ZERO {
                continue; // pinned particle
            }
            let acceleration = force * inverse_mass + self.gravity;
            body.velocities[i] = body.velocities[i] + acceleration * dt;
            body.positions[i] = body.positions[i] + body.velocities[i] * dt;
            self.collide_with_ground(body, i);
        }
    }

    /// Pushes particle `index` out of the ground plane and reflects its
    /// normal velocity by the restitution, if it penetrates.
    fn collide_with_ground(&self, body: &mut SoftBody<F>, index: usize) {
        let separation = self.ground.distance(body.positions[index]);
        if separation >= F::Scalar::ZERO {
            return;
        }
        body.positions[index] = body.positions[index] - self.ground.normal * separation;
        let normal_speed = body.velocities[index].dot(self.ground.normal);
        if normal_speed < F::Scalar::ZERO {
            body.velocities[index] = body.velocities[index]
                - self.ground.normal * (normal_speed * (F::Scalar::ONE + self.restitution));
        }
    }
}

/// Hooke's-law spring force plus velocity-relative damping on every
/// particle, from every spring of `body`.
fn spring_forces<F: GaFlavor>(body: &SoftBody<F>) -> Vec<F::Vector> {
    let mut forces = vec![F::Vector::ZERO; body.particle_count()];
    for spring in &body.springs {
        let delta = body.positions[spring.b] - body.positions[spring.a];
        let dist = delta.length();
        let direction = if dist > F::Scalar::EPSILON {
            delta * (F::Scalar::ONE / dist)
        } else {
            axis_x::<F>()
        };
        let stretch = dist - spring.rest_length;
        let spring_force = direction * (spring.stiffness * stretch);

        let relative_velocity = body.velocities[spring.b] - body.velocities[spring.a];
        let closing_speed = relative_velocity.dot(direction);
        let damping_force = direction * (spring.damping * closing_speed);

        let total = spring_force + damping_force;
        forces[spring.a] = forces[spring.a] + total;
        forces[spring.b] = forces[spring.b] - total;
    }
    forces
}

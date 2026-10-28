//! Semi-implicit Euler integration of rigid bodies.

use meridian_gac::{BivectorLike, GaFlavor, MotorLike, ScalarLike, VectorLike};

use super::RigidBody;

/// Advances rigid body velocity/angular velocity and position/orientation
/// by one timestep (semi-implicit Euler: velocity is updated by gravity
/// first, then position/orientation are updated using the *new* velocity
/// — more stable than explicit Euler for the same cost). Orientation is
/// integrated via `F::Bivector::exp` (rotor exponential map), not a naive
/// "add angle". Static bodies (`mass <= 0`) are untouched.
#[derive(Debug, Clone, Copy)]
pub struct Integrator<F: GaFlavor> {
    pub gravity: F::Vector,
}

impl<F: GaFlavor> Default for Integrator<F> {
    fn default() -> Self {
        Self {
            gravity: F::Vector::new(F::Scalar::ZERO, F::Scalar::from_f64(-9.81), F::Scalar::ZERO),
        }
    }
}

impl<F: GaFlavor> Integrator<F> {
    pub fn new(gravity: F::Vector) -> Self {
        Self { gravity }
    }

    pub fn step(&self, bodies: &mut [RigidBody<F>], dt: F::Scalar) {
        for body in bodies.iter_mut() {
            if body.inverse_mass() <= F::Scalar::ZERO {
                continue;
            }
            body.velocity = body.velocity + self.gravity * dt;
            // No torque source here yet (gravity acts at the center of
            // mass, so it never torques a body), so angular_velocity is
            // untouched by gravity — only ConstraintSolver changes it.
            let rotor_delta = (body.angular_velocity * dt).exp();
            let linear_delta = body.velocity * dt;
            let motor_delta = F::Motor::from_rotation_translation(rotor_delta, linear_delta);
            body.frame = body.frame.compose(motor_delta);
        }
    }
}

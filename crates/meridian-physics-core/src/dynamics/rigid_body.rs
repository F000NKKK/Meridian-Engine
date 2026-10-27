//! A rigid body: frame, motion state and collider.

use crate::collision::ColliderShape;
use crate::collision::aabb_of;
use meridian_gac_core::volume::{Aabb, Obb};
use meridian_gac_core::{BivectorLike, GaFlavor, MotorLike, ScalarLike, VectorLike};

/// A simulated rigid body: spatial frame (shared with every other
/// subsystem via the GAC) + linear and angular state. `mass <= 0` means
/// static/immovable (infinite mass) — never touched by [`Integrator`](crate::dynamics::Integrator) or
/// given any velocity change by [`ConstraintSolver`](crate::dynamics::ConstraintSolver).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RigidBody<F: GaFlavor> {
    pub frame: F::Motor,
    pub velocity: F::Vector,
    pub angular_velocity: F::Bivector,
    pub mass: F::Scalar,
    pub shape: ColliderShape<F>,
}

impl<F: GaFlavor> Default for RigidBody<F> {
    fn default() -> Self {
        Self {
            frame: F::Motor::identity(),
            velocity: F::Vector::ZERO,
            angular_velocity: F::Bivector::ZERO,
            mass: F::Scalar::ZERO,
            shape: ColliderShape::default(),
        }
    }
}

impl<F: GaFlavor> RigidBody<F> {
    pub fn inverse_mass(&self) -> F::Scalar {
        if self.mass > F::Scalar::ZERO {
            F::Scalar::ONE / self.mass
        } else {
            F::Scalar::ZERO
        }
    }

    /// Moment of inertia about the body's center of mass. `Sphere` is
    /// exact (`(2/5) * m * r²`, isotropic by construction). `Cuboid` is
    /// *not* exact — see `docs/physics-design.md`; this returns the
    /// average of the three principal moments, `(2/9) * m * (hx² + hy² +
    /// hz²)`, since [`ConstraintSolver`](crate::dynamics::ConstraintSolver)'s angular response only has a
    /// single scalar `inverse_inertia` to work with.
    pub fn moment_of_inertia(&self) -> F::Scalar {
        match self.shape {
            ColliderShape::Sphere { radius } => {
                F::Scalar::from_f64(0.4) * self.mass * radius * radius
            }
            ColliderShape::Cuboid { half_extents } => {
                F::Scalar::from_f64(2.0 / 9.0)
                    * self.mass
                    * (half_extents.x() * half_extents.x()
                        + half_extents.y() * half_extents.y()
                        + half_extents.z() * half_extents.z())
            }
        }
    }

    /// This body's collider as a world-space [`Obb`] — meaningful for
    /// `Cuboid` bodies (a `Sphere` has no orientation to speak of, so
    /// there's no equivalent method for it; [`RigidBody::position`] plus
    /// the collider's radius is all a sphere needs).
    pub fn as_obb(&self, half_extents: F::Vector) -> Obb<F> {
        Obb {
            frame: self.frame,
            half_extents,
        }
    }

    pub fn inverse_inertia(&self) -> F::Scalar {
        let i = self.moment_of_inertia();
        if i > F::Scalar::ZERO {
            F::Scalar::ONE / i
        } else {
            F::Scalar::ZERO
        }
    }

    pub fn position(&self) -> F::Vector {
        self.frame.transform_point(F::Vector::ZERO)
    }

    /// This body's world-space [`Aabb`] — the same bound
    /// [`BroadPhase::find_candidate_pairs`](crate::collision::BroadPhase::find_candidate_pairs) uses internally, exposed so
    /// an external batching adapter (e.g.
    /// `meridian-physics-compute`'s own broad-phase kernel) can reuse
    /// the real bound instead of re-deriving it.
    pub fn aabb(&self) -> Aabb<F> {
        aabb_of(self)
    }
}

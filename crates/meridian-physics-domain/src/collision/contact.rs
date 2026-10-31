//! A contact between two bodies.

use meridian_gac::{GaFlavor, ScalarLike, VectorLike};

/// A single narrow-phase contact between two colliders (indices into the
/// same `&[RigidBody]` slice `BroadPhase`/`NarrowPhase` were given).
/// `normal` points from `a` toward `b`; `point` is the contact point in
/// world space, used by [`ConstraintSolver`](crate::dynamics::ConstraintSolver) to compute torque.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Contact<F: GaFlavor> {
    /// Index of the first body in the input body slice.
    pub a: usize,
    /// Index of the second body in the input body slice.
    pub b: usize,
    /// Unit contact normal pointing from `a` toward `b`.
    pub normal: F::Vector,
    /// Overlap depth along `normal`.
    pub penetration: F::Scalar,
    /// World-space contact point.
    pub point: F::Vector,
    /// `true` for a box-box manifold point (see
    /// [`NarrowPhase::generate_contacts`](crate::collision::NarrowPhase::generate_contacts)'s `Cuboid`-vs-`Cuboid` branch):
    /// [`ConstraintSolver::resolve`](crate::dynamics::ConstraintSolver::resolve) applies linear impulse/friction and
    /// positional correction as usual but skips the torque/angular-
    /// velocity update entirely. The angular response is explicitly a
    /// "simplified, not fully coupled" single-scalar-inertia model (see
    /// `ConstraintSolver`'s own doc comment), only proven inert for
    /// sphere-sphere contacts; feeding it 4 independent per-corner torque
    /// computations per box manifold (each corner's offset differs, none
    /// of them solved as a coupled system) doesn't converge to rest —
    /// it compounds into runaway spin, the concrete "cube/pyramid spin
    /// hysterically" bug this suppresses. Always `false` for
    /// sphere-sphere/sphere-cuboid contacts, where the single contact
    /// point's angular response is stable.
    pub suppress_angular_response: bool,
}

impl<F: GaFlavor> Default for Contact<F> {
    fn default() -> Self {
        Self {
            a: 0,
            b: 0,
            normal: F::Vector::ZERO,
            penetration: F::Scalar::ZERO,
            point: F::Vector::ZERO,
            suppress_angular_response: false,
        }
    }
}

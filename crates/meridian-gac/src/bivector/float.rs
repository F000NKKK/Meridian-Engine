//! The float (`f32`) bivector: angular velocity and rotation generators.

use core::ops::{Add, Mul, Sub};

use meridian_numeric::{Fixed, Scalar};

use super::fixed::FixedBivector3;
use crate::flavor::BivectorLike;
use crate::rotor::float::Rotor;
use crate::vector::float::Vec3;

/// A bivector in the Euclidean part of the algebra (`e23`, `e31`, `e12`
/// components) — the GA-native representation of angular velocity and
/// rotation generators. Deliberately a distinct type from [`Vec3`]:
/// angular velocity lives in the Lie algebra so(3), which *is* the space
/// of these bivectors, not the space of vectors. Cross-product-based
/// "angular velocity as a vector" conflates the two because in 3D a
/// bivector's Hodge dual happens to have the same 3 components as a
/// vector — a coincidence special to 3D that GA makes explicit instead of
/// hiding (see [`Bivector3::wedge`]).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Bivector3 {
    pub e23: Scalar,
    pub e31: Scalar,
    pub e12: Scalar,
}

impl Bivector3 {
    pub const ZERO: Bivector3 = Bivector3 {
        e23: 0.0,
        e31: 0.0,
        e12: 0.0,
    };

    pub const fn new(e23: Scalar, e31: Scalar, e12: Scalar) -> Self {
        Self { e23, e31, e12 }
    }

    /// The wedge product `a ∧ b`, e.g. torque = `Bivector3::wedge(r, f)`
    /// for a force `f` applied at offset `r` from a pivot. Numerically
    /// identical to `a.cross(b)` — the "coincidence" `Bivector3`'s own
    /// doc comment mentions — but returns a `Bivector3`, not a `Vec3`,
    /// because a torque *is* a bivector quantity, not a vector one.
    pub fn wedge(a: Vec3, b: Vec3) -> Bivector3 {
        Bivector3::new(
            a.y * b.z - a.z * b.y,
            a.z * b.x - a.x * b.z,
            a.x * b.y - a.y * b.x,
        )
    }

    pub fn length(self) -> Scalar {
        (self.e23 * self.e23 + self.e31 * self.e31 + self.e12 * self.e12).sqrt()
    }

    /// `exp(B)` for a rotation-generator bivector: reduces to
    /// [`Rotor::from_axis_angle`] (a rotation-only bivector's exponential
    /// *is* a rotor, by definition — no separate derivation needed, this
    /// delegates to the already-verified formula) with axis = the
    /// bivector's direction and angle = its magnitude. Used to integrate
    /// a rigid body's orientation over a timestep: `(angular_velocity *
    /// dt).exp()` gives the rotor to compose onto the body's frame — the
    /// GA equivalent of quaternion exponential-map integration, and for
    /// the same reason: it stays exactly on the unit-rotor manifold, no
    /// drift/renormalization the way separately integrating three Euler
    /// angles would.
    pub fn exp(self) -> Rotor {
        let angle = self.length();
        if angle <= meridian_numeric::EPSILON {
            return Rotor::identity();
        }
        let axis = Vec3::new(self.e23, self.e31, self.e12) * (1.0 / angle);
        Rotor::from_axis_angle(axis, angle)
    }
}

impl Add for Bivector3 {
    type Output = Bivector3;
    fn add(self, rhs: Bivector3) -> Bivector3 {
        Bivector3::new(self.e23 + rhs.e23, self.e31 + rhs.e31, self.e12 + rhs.e12)
    }
}

impl Sub for Bivector3 {
    type Output = Bivector3;
    fn sub(self, rhs: Bivector3) -> Bivector3 {
        Bivector3::new(self.e23 - rhs.e23, self.e31 - rhs.e31, self.e12 - rhs.e12)
    }
}

impl Mul<Scalar> for Bivector3 {
    type Output = Bivector3;
    fn mul(self, rhs: Scalar) -> Bivector3 {
        Bivector3::new(self.e23 * rhs, self.e31 * rhs, self.e12 * rhs)
    }
}

impl BivectorLike for Bivector3 {
    type Scalar = Scalar;
    type Vector = Vec3;
    type Rotor = Rotor;

    const ZERO: Self = Bivector3::ZERO;

    fn wedge(a: Vec3, b: Vec3) -> Self {
        Bivector3::wedge(a, b)
    }
    fn length(self) -> Scalar {
        self.length()
    }
    fn exp(self) -> Rotor {
        self.exp()
    }
}

impl Bivector3 {
    /// Deliberate precision-changing cast to the deterministic flavor.
    pub fn to_fixed_lossy(self) -> FixedBivector3 {
        FixedBivector3::new(
            Fixed::from_num(self.e23 as f64),
            Fixed::from_num(self.e31 as f64),
            Fixed::from_num(self.e12 as f64),
        )
    }
}

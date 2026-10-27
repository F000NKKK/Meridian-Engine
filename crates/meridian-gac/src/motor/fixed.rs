//! The fixed-point (`Fixed`) motor, mirroring [`super::float`].

use meridian_numeric::Fixed;

use super::float::Motor3;
use super::frame;
use crate::blade;
use crate::flavor::MotorLike;
use crate::flavor::fixed::FixedFlavor;
use crate::multivector::fixed::FixedMultivector;
use crate::multivector::float::Multivector;
use crate::rotor::fixed::FixedRotor;
use crate::vector::fixed::FixedVec3;

/// Mirrors [`crate::Motor3`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedMotor3(pub FixedMultivector);

impl Default for FixedMotor3 {
    fn default() -> Self {
        FixedMotor3::identity()
    }
}

impl FixedMotor3 {
    pub fn identity() -> Self {
        FixedMotor3(FixedMultivector::scalar(Fixed::ONE))
    }

    pub fn translation(t: FixedVec3) -> Self {
        let mut m = [Fixed::ZERO; 16];
        m[blade::S] = Fixed::ONE;
        let half = Fixed::from_num(0.5);
        m[blade::E01] = -t.x * half;
        m[blade::E02] = -t.y * half;
        m[blade::E03] = -t.z * half;
        FixedMotor3(FixedMultivector(m))
    }

    pub fn from_rotation_translation(rotor: FixedRotor, t: FixedVec3) -> Self {
        FixedMotor3::translation(t).compose_raw(FixedMotor3(rotor.0))
    }

    pub fn compose(self, rhs: FixedMotor3) -> FixedMotor3 {
        FixedMotor3(rhs.0 * self.0)
    }

    fn compose_raw(self, rhs: FixedMotor3) -> FixedMotor3 {
        FixedMotor3(self.0 * rhs.0)
    }

    pub fn inverse(self) -> FixedMotor3 {
        FixedMotor3(self.0.reverse())
    }

    pub fn transform_point(self, point: FixedVec3) -> FixedVec3 {
        let mut pm = [Fixed::ZERO; 16];
        pm[blade::E123] = Fixed::ONE;
        pm[blade::E023] = -point.x;
        pm[blade::E013] = point.y;
        pm[blade::E012] = -point.z;
        let p = FixedMultivector(pm);
        let r = self.0 * p * self.0.reverse();
        FixedVec3::new(-r.0[blade::E023], r.0[blade::E013], -r.0[blade::E012])
    }

    /// Mirrors [`crate::Motor3::transform_vector`]: the
    /// rotation-only action on a direction, computed as
    /// `transform_point(v) - transform_point(ZERO)` so translation
    /// cancels out exactly.
    pub fn transform_vector(self, v: FixedVec3) -> FixedVec3 {
        self.transform_point(v) - self.transform_point(FixedVec3::ZERO)
    }
}

impl MotorLike for FixedMotor3 {
    type Scalar = Fixed;
    type Vector = FixedVec3;
    type Rotor = FixedRotor;

    fn identity() -> Self {
        FixedMotor3::identity()
    }
    fn translation(t: FixedVec3) -> Self {
        FixedMotor3::translation(t)
    }
    fn from_rotation_translation(rotor: FixedRotor, t: FixedVec3) -> Self {
        FixedMotor3::from_rotation_translation(rotor, t)
    }
    fn compose(self, rhs: Self) -> Self {
        self.compose(rhs)
    }
    fn inverse(self) -> Self {
        self.inverse()
    }
    fn transform_point(self, p: FixedVec3) -> FixedVec3 {
        self.transform_point(p)
    }
    fn transform_vector(self, v: FixedVec3) -> FixedVec3 {
        self.transform_vector(v)
    }
}

impl FixedMotor3 {
    /// Deliberate precision-changing cast to the default flavor —
    /// component-wise across the underlying multivector, not a
    /// re-derivation through rotation/translation. The way a
    /// `physics-core::fixed::RigidBody`'s pose reaches rendering/ECS/audio,
    /// which stay `f32` regardless of which physics path produced the pose.
    pub fn to_float_lossy(self) -> Motor3 {
        let mut out = [0.0f32; 16];
        for (dst, src) in out.iter_mut().zip(self.0.0) {
            *dst = src.to_num() as f32;
        }
        Motor3(Multivector(out))
    }
}

/// A named reference frame in the fixed-point flavor.
pub type FixedFrame = frame::Frame<FixedFlavor>;

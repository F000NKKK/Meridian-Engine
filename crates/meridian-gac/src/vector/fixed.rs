//! The fixed-point (`Fixed`) 3D vector, mirroring [`super::float`].

use core::ops::{Add, Mul, Neg, Sub};

use meridian_numeric::{Fixed, Scalar};

use super::float::Vec3;
use crate::flavor::VectorLike;
use crate::flavor::fixed::{FIXED_EPSILON, FixedFlavor};
use crate::volume::Shape;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FixedVec3 {
    pub x: Fixed,
    pub y: Fixed,
    pub z: Fixed,
}

impl FixedVec3 {
    pub const ZERO: FixedVec3 = FixedVec3 {
        x: Fixed::ZERO,
        y: Fixed::ZERO,
        z: Fixed::ZERO,
    };

    pub const fn new(x: Fixed, y: Fixed, z: Fixed) -> Self {
        Self { x, y, z }
    }

    pub fn dot(self, rhs: FixedVec3) -> Fixed {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    pub fn cross(self, rhs: FixedVec3) -> FixedVec3 {
        FixedVec3::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    pub fn length_squared(self) -> Fixed {
        self.dot(self)
    }

    pub fn length(self) -> Fixed {
        self.length_squared().sqrt()
    }

    pub fn normalize(self) -> FixedVec3 {
        let len = self.length();
        if len <= FIXED_EPSILON {
            self
        } else {
            self * (Fixed::ONE / len)
        }
    }
}

impl Add for FixedVec3 {
    type Output = FixedVec3;
    fn add(self, rhs: FixedVec3) -> FixedVec3 {
        FixedVec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for FixedVec3 {
    type Output = FixedVec3;
    fn sub(self, rhs: FixedVec3) -> FixedVec3 {
        FixedVec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Neg for FixedVec3 {
    type Output = FixedVec3;
    fn neg(self) -> FixedVec3 {
        FixedVec3::new(-self.x, -self.y, -self.z)
    }
}

impl Mul<Fixed> for FixedVec3 {
    type Output = FixedVec3;
    fn mul(self, rhs: Fixed) -> FixedVec3 {
        FixedVec3::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl VectorLike for FixedVec3 {
    type Scalar = Fixed;

    const ZERO: Self = FixedVec3::ZERO;

    fn new(x: Fixed, y: Fixed, z: Fixed) -> Self {
        FixedVec3::new(x, y, z)
    }
    fn x(self) -> Fixed {
        self.x
    }
    fn y(self) -> Fixed {
        self.y
    }
    fn z(self) -> Fixed {
        self.z
    }
    fn dot(self, rhs: Self) -> Fixed {
        self.dot(rhs)
    }
    fn cross(self, rhs: Self) -> Self {
        self.cross(rhs)
    }
    fn length(self) -> Fixed {
        self.length()
    }
    fn length_squared(self) -> Fixed {
        self.length_squared()
    }
    fn normalize(self) -> Self {
        self.normalize()
    }
}

/// A single point, treated as a degenerate shape — see
/// the float flavor's matching impl for why this is concrete, not a blanket
/// impl in `lib.rs`.
impl Shape<FixedFlavor> for FixedVec3 {
    fn support(&self, _direction: FixedVec3) -> FixedVec3 {
        *self
    }
}

impl FixedVec3 {
    /// Deliberate precision-changing cast to the default flavor — needed
    /// for GPU handoff (GPUs are `f32`-native; see
    /// `docs/adr/008-fixed-point-determinism.md`) or any other `f32`-only
    /// consumer (rendering, ECS, audio).
    pub fn to_float_lossy(self) -> Vec3 {
        Vec3::new(
            self.x.to_num() as f32,
            self.y.to_num() as f32,
            self.z.to_num() as f32,
        )
    }
}

impl Add<Vec3> for FixedVec3 {
    type Output = FixedVec3;
    fn add(self, rhs: Vec3) -> FixedVec3 {
        self + rhs.to_fixed_lossy()
    }
}

impl Sub<Vec3> for FixedVec3 {
    type Output = FixedVec3;
    fn sub(self, rhs: Vec3) -> FixedVec3 {
        self - rhs.to_fixed_lossy()
    }
}

impl Mul<Scalar> for FixedVec3 {
    type Output = FixedVec3;
    fn mul(self, rhs: Scalar) -> FixedVec3 {
        self * Fixed::from_num(rhs as f64)
    }
}

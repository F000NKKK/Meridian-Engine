//! The float (`f32`) 3D Euclidean vector.

use core::ops::{Add, Mul, Neg, Sub};

use meridian_numeric::{Fixed, Scalar};

use super::fixed::FixedVec3;
use crate::flavor::VectorLike;
use crate::flavor::float::FloatFlavor;
use crate::volume::Shape;

/// A plain 3D Euclidean vector — the ergonomic type most call sites use
/// (points, directions, axes, translations); [`Multivector`](crate::Multivector)/[`Motor3`](crate::Motor3)
/// are the machinery underneath, not the day-to-day API.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Vec3 {
    /// X coordinate.
    pub x: Scalar,
    /// Y coordinate.
    pub y: Scalar,
    /// Z coordinate.
    pub z: Scalar,
}

impl Vec3 {
    /// The zero vector.
    pub const ZERO: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    /// The unit X axis.
    pub const X: Vec3 = Vec3 {
        x: 1.0,
        y: 0.0,
        z: 0.0,
    };
    /// The unit Y axis.
    pub const Y: Vec3 = Vec3 {
        x: 0.0,
        y: 1.0,
        z: 0.0,
    };
    /// The unit Z axis.
    pub const Z: Vec3 = Vec3 {
        x: 0.0,
        y: 0.0,
        z: 1.0,
    };

    /// Constructs a vector from its coordinates.
    pub const fn new(x: Scalar, y: Scalar, z: Scalar) -> Self {
        Self { x, y, z }
    }

    /// Computes the dot product with `rhs`.
    pub fn dot(self, rhs: Vec3) -> Scalar {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    /// Computes the cross product with `rhs`.
    pub fn cross(self, rhs: Vec3) -> Vec3 {
        Vec3::new(
            self.y * rhs.z - self.z * rhs.y,
            self.z * rhs.x - self.x * rhs.z,
            self.x * rhs.y - self.y * rhs.x,
        )
    }

    /// Returns the squared Euclidean length.
    pub fn length_squared(self) -> Scalar {
        self.dot(self)
    }

    /// Returns the Euclidean length.
    pub fn length(self) -> Scalar {
        self.length_squared().sqrt()
    }

    /// Returns `self` unchanged if it's shorter than
    /// [`meridian_numeric::EPSILON`] rather than dividing by ~zero.
    pub fn normalize(self) -> Vec3 {
        let len = self.length();
        if len <= meridian_numeric::EPSILON {
            self
        } else {
            self * (1.0 / len)
        }
    }

    #[cfg(test)]
    pub(crate) fn approx_eq(self, rhs: Vec3) -> bool {
        meridian_numeric::approx_eq(self.x, rhs.x)
            && meridian_numeric::approx_eq(self.y, rhs.y)
            && meridian_numeric::approx_eq(self.z, rhs.z)
    }
}

impl Add for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x + rhs.x, self.y + rhs.y, self.z + rhs.z)
    }
}

impl Sub for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: Vec3) -> Vec3 {
        Vec3::new(self.x - rhs.x, self.y - rhs.y, self.z - rhs.z)
    }
}

impl Neg for Vec3 {
    type Output = Vec3;
    fn neg(self) -> Vec3 {
        Vec3::new(-self.x, -self.y, -self.z)
    }
}

impl Mul<Scalar> for Vec3 {
    type Output = Vec3;
    fn mul(self, rhs: Scalar) -> Vec3 {
        Vec3::new(self.x * rhs, self.y * rhs, self.z * rhs)
    }
}

impl VectorLike for Vec3 {
    type Scalar = Scalar;

    const ZERO: Self = Vec3::ZERO;

    fn new(x: Scalar, y: Scalar, z: Scalar) -> Self {
        Vec3::new(x, y, z)
    }
    fn x(self) -> Scalar {
        self.x
    }
    fn y(self) -> Scalar {
        self.y
    }
    fn z(self) -> Scalar {
        self.z
    }
    fn dot(self, rhs: Self) -> Scalar {
        self.dot(rhs)
    }
    fn cross(self, rhs: Self) -> Self {
        self.cross(rhs)
    }
    fn length(self) -> Scalar {
        self.length()
    }
    fn length_squared(self) -> Scalar {
        self.length_squared()
    }
    fn normalize(self) -> Self {
        self.normalize()
    }
}

/// A single point, treated as a degenerate shape: its own support point
/// regardless of direction. See `crate::Shape`'s doc comment; this can't
/// be a blanket `impl<F: GaFlavor> Shape<F> for F::Vector` in `lib.rs` for
/// coherence reasons (see the comment there), so it's concrete per flavor
/// instead.
impl Shape<FloatFlavor> for Vec3 {
    fn support(&self, _direction: Vec3) -> Vec3 {
        *self
    }
}

impl Vec3 {
    /// Deliberate precision-changing cast to the deterministic flavor.
    pub fn to_fixed_lossy(self) -> FixedVec3 {
        FixedVec3::new(
            Fixed::from_num(self.x as f64),
            Fixed::from_num(self.y as f64),
            Fixed::from_num(self.z as f64),
        )
    }
}

impl Add<FixedVec3> for Vec3 {
    type Output = Vec3;
    fn add(self, rhs: FixedVec3) -> Vec3 {
        self + rhs.to_float_lossy()
    }
}

impl Sub<FixedVec3> for Vec3 {
    type Output = Vec3;
    fn sub(self, rhs: FixedVec3) -> Vec3 {
        self - rhs.to_float_lossy()
    }
}

impl Mul<Fixed> for Vec3 {
    type Output = Vec3;
    fn mul(self, rhs: Fixed) -> Vec3 {
        self * (rhs.to_num() as f32)
    }
}

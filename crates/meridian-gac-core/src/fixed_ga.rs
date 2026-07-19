//! A fixed-point (`Fixed`, Q16.16) mirror of [`crate::float_ga`]'s PGA
//! machinery — `Multivector`/`Vec3`/`Bivector3`/`Rotor`/`Motor3` — for
//! simulation state that must reproduce bit-identically across platforms
//! (see `meridian_numeric_core::Fixed`'s doc comment for why: lockstep
//! networking, replay).
//!
//! This is a deliberate, disclosed duplication of [`crate::float_ga`]'s
//! structure, not an oversight: that module is hardcoded to `Scalar =
//! f32`, and `f32` cannot give bit-identical results across
//! platforms/compilers. Making the *default* `float_ga` types generic
//! over the scalar type instead of duplicating was considered and
//! rejected: `gac-core`'s compute-batching story
//! (`meridian-gac-compute`, dispatched to GPU via `compute-runtime`) has
//! no good answer for fixed-point at all — GPUs are `f32`-native
//! hardware with no real `i64` support, so a GPU-dispatchable `Motor3`
//! has to stay floating-point regardless, and a generic `Motor3<S>`
//! would still need this exact duplication at the instantiation site to
//! keep the GPU path pure `f32`. Every operation here mirrors its
//! `float_ga` counterpart function-for-function so the two stay easy to
//! compare and keep in sync by inspection.
//!
//! This is deterministic-simulation opt-in, not the default —
//! `float_ga`'s `Motor3` (`f32`) remains the everyday, GPU-dispatchable
//! pose type for everything else (rendering, ECS, audio, and
//! `physics-core`'s own default `RigidBody`). But it's *usable by any
//! crate* that needs CPU-deterministic geometry (`physics-core`'s
//! `DeterministicBody` today, potentially a deterministic
//! `graphics-core` CPU path or a large precise CPU/GPU-emulated
//! simulation later) — that reusability is exactly why the primitives
//! below live here in `gac-core`, not inside `physics-core`, the same
//! reason `float_ga`'s primitives aren't `physics-core`-local either.

use core::ops::{Add, Mul, Neg, Sub};
use meridian_numeric_core::Fixed;

use crate::blade;

/// A small tolerance a few Q16.16 steps above zero, used the same way
/// [`meridian_numeric_core::EPSILON`] is used in [`crate::float_ga`] —
/// degenerate (near-zero-length) vector/bivector guards.
const FIXED_EPSILON: Fixed = Fixed::from_bits(4); // 4 / 65536 ~= 6.1e-5

fn basis_product(a: u8, b: u8) -> (Fixed, u8) {
    let mut seq: [u8; 8] = [0; 8];
    let mut len = 0usize;
    for i in 0..4u8 {
        if a & (1 << i) != 0 {
            seq[len] = i;
            len += 1;
        }
    }
    for i in 0..4u8 {
        if b & (1 << i) != 0 {
            seq[len] = i;
            len += 1;
        }
    }

    let mut sign = Fixed::ONE;
    loop {
        let mut swapped = false;
        for i in 0..len.saturating_sub(1) {
            if seq[i] > seq[i + 1] {
                seq.swap(i, i + 1);
                sign = -sign;
                swapped = true;
            }
        }

        let mut contracted = false;
        let mut i = 0;
        while i + 1 < len {
            if seq[i] == seq[i + 1] {
                if seq[i] == 0 {
                    return (Fixed::ZERO, 0);
                }
                for k in i..len - 2 {
                    seq[k] = seq[k + 2];
                }
                len -= 2;
                contracted = true;
            } else {
                i += 1;
            }
        }

        if !swapped && !contracted {
            break;
        }
    }

    let mut mask = 0u8;
    for &b in seq.iter().take(len) {
        mask |= 1 << b;
    }
    (sign, mask)
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedMultivector(pub [Fixed; 16]);

impl FixedMultivector {
    pub const ZERO: FixedMultivector = FixedMultivector([Fixed::ZERO; 16]);

    pub fn scalar(s: Fixed) -> Self {
        let mut m = [Fixed::ZERO; 16];
        m[blade::S] = s;
        FixedMultivector(m)
    }

    pub fn reverse(self) -> Self {
        let mut out = self.0;
        for (i, v) in out.iter_mut().enumerate() {
            let k = (i as u32).count_ones();
            if matches!(k, 2 | 3) {
                *v = -*v;
            }
        }
        FixedMultivector(out)
    }
}

impl Add for FixedMultivector {
    type Output = FixedMultivector;
    fn add(self, rhs: FixedMultivector) -> FixedMultivector {
        let mut out = self.0;
        for (o, r) in out.iter_mut().zip(rhs.0) {
            *o = *o + r;
        }
        FixedMultivector(out)
    }
}

impl Sub for FixedMultivector {
    type Output = FixedMultivector;
    fn sub(self, rhs: FixedMultivector) -> FixedMultivector {
        let mut out = self.0;
        for (o, r) in out.iter_mut().zip(rhs.0) {
            *o = *o - r;
        }
        FixedMultivector(out)
    }
}

impl Neg for FixedMultivector {
    type Output = FixedMultivector;
    fn neg(self) -> FixedMultivector {
        let mut out = self.0;
        for v in &mut out {
            *v = -*v;
        }
        FixedMultivector(out)
    }
}

impl Mul for FixedMultivector {
    type Output = FixedMultivector;
    fn mul(self, rhs: FixedMultivector) -> FixedMultivector {
        let mut out = [Fixed::ZERO; 16];
        for a in 0..16u8 {
            let lhs_val = self.0[a as usize];
            if lhs_val == Fixed::ZERO {
                continue;
            }
            for b in 0..16u8 {
                let rhs_val = rhs.0[b as usize];
                if rhs_val == Fixed::ZERO {
                    continue;
                }
                let (sign, mask) = basis_product(a, b);
                if sign == Fixed::ZERO {
                    continue;
                }
                out[mask as usize] = out[mask as usize] + sign * lhs_val * rhs_val;
            }
        }
        FixedMultivector(out)
    }
}

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

/// Mirrors [`crate::float_ga::Bivector3`] — see that type's doc comment
/// for why angular quantities are bivectors, not vectors.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FixedBivector3 {
    pub e23: Fixed,
    pub e31: Fixed,
    pub e12: Fixed,
}

impl FixedBivector3 {
    pub const ZERO: FixedBivector3 = FixedBivector3 {
        e23: Fixed::ZERO,
        e31: Fixed::ZERO,
        e12: Fixed::ZERO,
    };

    pub const fn new(e23: Fixed, e31: Fixed, e12: Fixed) -> Self {
        Self { e23, e31, e12 }
    }

    pub fn wedge(a: FixedVec3, b: FixedVec3) -> FixedBivector3 {
        FixedBivector3::new(
            a.y * b.z - a.z * b.y,
            a.z * b.x - a.x * b.z,
            a.x * b.y - a.y * b.x,
        )
    }

    pub fn length(self) -> Fixed {
        (self.e23 * self.e23 + self.e31 * self.e31 + self.e12 * self.e12).sqrt()
    }

    pub fn exp(self) -> FixedRotor {
        let angle = self.length();
        if angle <= FIXED_EPSILON {
            return FixedRotor::identity();
        }
        let axis = FixedVec3::new(self.e23, self.e31, self.e12) * (Fixed::ONE / angle);
        FixedRotor::from_axis_angle(axis, angle)
    }
}

impl Add for FixedBivector3 {
    type Output = FixedBivector3;
    fn add(self, rhs: FixedBivector3) -> FixedBivector3 {
        FixedBivector3::new(self.e23 + rhs.e23, self.e31 + rhs.e31, self.e12 + rhs.e12)
    }
}

impl Sub for FixedBivector3 {
    type Output = FixedBivector3;
    fn sub(self, rhs: FixedBivector3) -> FixedBivector3 {
        FixedBivector3::new(self.e23 - rhs.e23, self.e31 - rhs.e31, self.e12 - rhs.e12)
    }
}

impl Mul<Fixed> for FixedBivector3 {
    type Output = FixedBivector3;
    fn mul(self, rhs: Fixed) -> FixedBivector3 {
        FixedBivector3::new(self.e23 * rhs, self.e31 * rhs, self.e12 * rhs)
    }
}

/// Mirrors [`crate::float_ga::Rotor`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedRotor(pub FixedMultivector);

impl Default for FixedRotor {
    fn default() -> Self {
        FixedRotor::identity()
    }
}

impl FixedRotor {
    pub fn identity() -> Self {
        FixedRotor(FixedMultivector::scalar(Fixed::ONE))
    }

    pub fn from_axis_angle(axis: FixedVec3, angle: Fixed) -> Self {
        let axis = axis.normalize();
        let half = angle * Fixed::from_num(0.5);
        let (s, c) = half.sin_cos();
        let mut m = [Fixed::ZERO; 16];
        m[blade::S] = c;
        m[blade::E23] = -s * axis.x;
        m[blade::E13] = s * axis.y;
        m[blade::E12] = -s * axis.z;
        FixedRotor(FixedMultivector(m))
    }

    pub fn compose(self, rhs: FixedRotor) -> FixedRotor {
        FixedRotor(rhs.0 * self.0)
    }

    pub fn reverse(self) -> FixedRotor {
        FixedRotor(self.0.reverse())
    }

    pub fn transform_vector(self, v: FixedVec3) -> FixedVec3 {
        let mut vm = [Fixed::ZERO; 16];
        vm[blade::E1] = v.x;
        vm[blade::E2] = v.y;
        vm[blade::E3] = v.z;
        let p = FixedMultivector(vm);
        let r = self.0 * p * self.0.reverse();
        FixedVec3::new(r.0[blade::E1], r.0[blade::E2], r.0[blade::E3])
    }
}

/// Mirrors [`crate::float_ga::Motor3`].
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

    /// Mirrors [`crate::float_ga::Motor3::transform_vector`]: the
    /// rotation-only action on a direction, computed as
    /// `transform_point(v) - transform_point(ZERO)` so translation
    /// cancels out exactly.
    pub fn transform_vector(self, v: FixedVec3) -> FixedVec3 {
        self.transform_point(v) - self.transform_point(FixedVec3::ZERO)
    }
}

/// Mirrors [`crate::float_ga::Shape`].
pub trait FixedShape {
    fn support(&self, direction: FixedVec3) -> FixedVec3;
}

/// Mirrors [`crate::float_ga::Shape`]'s `impl for Vec3`.
impl FixedShape for FixedVec3 {
    fn support(&self, _direction: FixedVec3) -> FixedVec3 {
        *self
    }
}

/// Mirrors [`crate::float_ga::Aabb`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FixedAabb {
    pub min: FixedVec3,
    pub max: FixedVec3,
}

impl FixedAabb {
    pub fn from_sphere(center: FixedVec3, radius: Fixed) -> Self {
        let r = FixedVec3::new(radius, radius, radius);
        Self {
            min: center - r,
            max: center + r,
        }
    }

    /// An axis-aligned cube: equal half-extent on every axis.
    pub fn cube(center: FixedVec3, half_extent: Fixed) -> Self {
        Self::from_sphere(center, half_extent)
    }

    pub fn overlaps(&self, other: &FixedAabb) -> bool {
        self.min.x <= other.max.x
            && self.max.x >= other.min.x
            && self.min.y <= other.max.y
            && self.max.y >= other.min.y
            && self.min.z <= other.max.z
            && self.max.z >= other.min.z
    }
}

impl FixedShape for FixedAabb {
    fn support(&self, direction: FixedVec3) -> FixedVec3 {
        FixedVec3::new(
            if direction.x >= Fixed::ZERO {
                self.max.x
            } else {
                self.min.x
            },
            if direction.y >= Fixed::ZERO {
                self.max.y
            } else {
                self.min.y
            },
            if direction.z >= Fixed::ZERO {
                self.max.z
            } else {
                self.min.z
            },
        )
    }
}

/// Mirrors [`crate::float_ga::Sphere`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FixedSphere {
    pub center: FixedVec3,
    pub radius: Fixed,
}

impl FixedShape for FixedSphere {
    fn support(&self, direction: FixedVec3) -> FixedVec3 {
        self.center + direction.normalize() * self.radius
    }
}

/// Mirrors [`crate::float_ga::Obb`].
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct FixedObb {
    pub frame: FixedMotor3,
    pub half_extents: FixedVec3,
}

impl FixedObb {
    /// An oriented cube: equal half-extent on every axis.
    pub fn cube(frame: FixedMotor3, half_extent: Fixed) -> Self {
        Self {
            frame,
            half_extents: FixedVec3::new(half_extent, half_extent, half_extent),
        }
    }
}

impl FixedShape for FixedObb {
    fn support(&self, direction: FixedVec3) -> FixedVec3 {
        let local_direction = self.frame.inverse().transform_vector(direction);
        let local_support = FixedVec3::new(
            self.half_extents.x * local_direction.x.signum(),
            self.half_extents.y * local_direction.y.signum(),
            self.half_extents.z * local_direction.z.signum(),
        );
        self.frame.transform_point(local_support)
    }
}

/// Mirrors [`crate::float_ga::Cone`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedCone {
    pub apex: FixedVec3,
    pub axis: FixedVec3,
    pub half_angle: Fixed,
    pub height: Fixed,
}

impl FixedShape for FixedCone {
    fn support(&self, direction: FixedVec3) -> FixedVec3 {
        let base_center = self.apex + self.axis * self.height;
        let base_radius = self.height * self.half_angle.tan();

        let along_axis = direction.dot(self.axis);
        let perpendicular = direction - self.axis * along_axis;
        let perpendicular_len = perpendicular.length();

        let rim_point = if perpendicular_len > FIXED_EPSILON {
            base_center + perpendicular * (base_radius / perpendicular_len)
        } else {
            base_center
        };

        if direction.dot(self.apex) >= direction.dot(rim_point) {
            self.apex
        } else {
            rim_point
        }
    }
}

/// Mirrors [`crate::float_ga::Plane`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FixedPlane {
    pub normal: FixedVec3,
    pub d: Fixed,
}

impl FixedPlane {
    pub fn normalize(self) -> Self {
        let len = self.normal.length();
        FixedPlane {
            normal: FixedVec3::new(self.normal.x / len, self.normal.y / len, self.normal.z / len),
            d: self.d / len,
        }
    }

    pub fn distance(&self, p: FixedVec3) -> Fixed {
        self.normal.dot(p) + self.d
    }

    pub fn contains<S: FixedShape>(&self, shape: &S) -> bool {
        self.distance(shape.support(self.normal)) >= Fixed::ZERO
    }
}

/// Mirrors [`crate::float_ga::ConvexVolume`].
#[derive(Debug, Clone, Default)]
pub struct FixedConvexVolume {
    pub planes: Vec<FixedPlane>,
}

impl FixedConvexVolume {
    pub fn new(planes: Vec<FixedPlane>) -> Self {
        Self { planes }
    }

    pub fn intersects<S: FixedShape>(&self, shape: &S) -> bool {
        self.planes.iter().all(|plane| plane.contains(shape))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::float_ga as float;

    fn fv3(x: f64, y: f64, z: f64) -> FixedVec3 {
        FixedVec3::new(Fixed::from_num(x), Fixed::from_num(y), Fixed::from_num(z))
    }

    fn assert_fixed_vec3_approx(got: FixedVec3, want: float::Vec3) {
        let tolerance = 1e-3; // CORDIC/multiply rounding through several ops.
        assert!(
            (got.x.to_num() - want.x as f64).abs() < tolerance
                && (got.y.to_num() - want.y as f64).abs() < tolerance
                && (got.z.to_num() - want.z as f64).abs() < tolerance,
            "expected ~{want:?}, got ({}, {}, {})",
            got.x.to_num(),
            got.y.to_num(),
            got.z.to_num()
        );
    }

    #[test]
    fn fixed_translation_matches_float_ga_oracle() {
        let t = float::Vec3::new(1.0, 2.0, 3.0);
        let p = float::Vec3::new(5.0, -1.0, 0.5);
        let want = float::Motor3::translation(t).transform_point(p);

        let ft = fv3(1.0, 2.0, 3.0);
        let fp = fv3(5.0, -1.0, 0.5);
        let got = FixedMotor3::translation(ft).transform_point(fp);

        assert_fixed_vec3_approx(got, want);
    }

    #[test]
    fn fixed_rotation_matches_float_ga_rodrigues_oracle() {
        let angle = 1.234_f32;
        let axis = float::Vec3::new(0.0, 0.0, 1.0);
        let p = float::Vec3::new(1.0, 0.0, 0.0);
        let want = float::Rotor::from_axis_angle(axis, angle).transform_vector(p);

        let fangle = Fixed::from_num(angle as f64);
        let faxis = fv3(0.0, 0.0, 1.0);
        let fp = fv3(1.0, 0.0, 0.0);
        let got = FixedRotor::from_axis_angle(faxis, fangle).transform_vector(fp);

        assert_fixed_vec3_approx(got, want);
    }

    #[test]
    fn fixed_rotation_translation_composition_matches_float_ga_oracle() {
        let rotor = float::Rotor::from_axis_angle(float::Vec3::new(0.0, 0.0, 1.0), 0.7);
        let t = float::Vec3::new(3.0, -2.0, 1.0);
        let p = float::Vec3::new(2.0, 1.0, 0.0);
        let want = float::Motor3::from_rotation_translation(rotor, t).transform_point(p);

        let frotor = FixedRotor::from_axis_angle(fv3(0.0, 0.0, 1.0), Fixed::from_num(0.7));
        let ft = fv3(3.0, -2.0, 1.0);
        let fp = fv3(2.0, 1.0, 0.0);
        let got = FixedMotor3::from_rotation_translation(frotor, ft).transform_point(fp);

        assert_fixed_vec3_approx(got, want);
    }

    #[test]
    fn fixed_motor_inverse_round_trips() {
        let rotor = FixedRotor::from_axis_angle(fv3(0.3, 0.6, 0.2), Fixed::from_num(0.9));
        let motor = FixedMotor3::from_rotation_translation(rotor, fv3(4.0, -1.0, 2.0));
        let p = fv3(1.0, 2.0, 3.0);

        let round_tripped = motor.inverse().transform_point(motor.transform_point(p));

        let tolerance = Fixed::from_num(1e-3);
        assert!((round_tripped.x - p.x).abs() < tolerance);
        assert!((round_tripped.y - p.y).abs() < tolerance);
        assert!((round_tripped.z - p.z).abs() < tolerance);
    }

    #[test]
    fn fixed_bivector_exp_matches_rotor_from_axis_angle_directly() {
        let bivector = FixedBivector3::new(Fixed::ZERO, Fixed::ZERO, Fixed::from_num(0.5));
        let via_exp = bivector.exp();
        let direct = FixedRotor::from_axis_angle(fv3(0.0, 0.0, 1.0), Fixed::from_num(0.5));

        let p = fv3(1.0, 0.0, 0.0);
        let a = via_exp.transform_vector(p);
        let b = direct.transform_vector(p);
        let tolerance = Fixed::from_num(1e-3);
        assert!((a.x - b.x).abs() < tolerance);
        assert!((a.y - b.y).abs() < tolerance);
        assert!((a.z - b.z).abs() < tolerance);
    }
}

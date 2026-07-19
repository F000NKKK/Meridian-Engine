//! Geometric Algebra Core — vectors, bivectors, multivectors, rotors and motors; the shared spatial math foundation for every other subsystem.
//!
//! Two flavors, in their own modules: [`float_ga`] (`f32`, the default —
//! re-exported at the crate root, so `meridian_gac_core::Vec3`/`Motor3`/
//! `Aabb`/etc. resolve to it unchanged) and [`fixed_ga`] (`Fixed`,
//! deterministic — usable by *any* crate that needs CPU-deterministic
//! geometry, not just `physics-core`). Only `Multivector`/`Vec3`/
//! `Bivector3`/`Rotor`/`Motor3` themselves are concretely duplicated
//! between the two (see [`fixed_ga`]'s module doc comment for why:
//! `meridian-gac-compute` dispatches them, and `compute-runtime` has no
//! GPU backend to dispatch to at all yet — CPU vs. GPU is a dispatch
//! *setting* a caller picks once one exists, not a type-level
//! restriction this crate imposes; a future GPU backend could in
//! principle run `Fixed` kernels too, at the cost of `i64` emulation and
//! losing the bit-exact determinism guarantee to GPU execution-order
//! nondeterminism — a tradeoff for the caller to accept knowingly, not
//! something forbidden here).
//!
//! Everything else — geometric primitives (`Aabb`, `Sphere`, `Obb`,
//! `Cone`, `Plane`, `Shape`, `ConvexVolume`, `Projection`, `Frame`) — is
//! written **once**, generic over [`GaFlavor`], because none of it has
//! `float_ga`/`fixed_ga`'s Motor3-specific GPU-dispatch constraint: an
//! AABB overlap test or a projection matrix derivation is the same
//! sequence of operations regardless of which scalar type it runs on, so
//! duplicating it would just be maintenance risk for no reason (see
//! `CLAUDE.md`'s "Float/Fixed branching" rule).  `float_ga`/`fixed_ga`
//! each expose thin type aliases (`float_ga::Aabb =
//! Aabb<FloatFlavor>`, `fixed_ga::FixedAabb = Aabb<FixedFlavor>`, ...)
//! so existing call sites don't need to think about the generic
//! parameter at all.
//!
//! See docs/gac-design.md and
//! [ADR 001](../../../docs/adr/001-geometric-algebra-as-spatial-model.md).

use core::ops::{Add, Div, Mul, Neg, Sub};

pub mod fixed_ga;
pub mod float_ga;

pub use float_ga::*;

/// Named blade indices into a `Multivector`'s 16 components, encoded as a
/// 4-bit mask over `{e0, e1, e2, e3}` (bit i set means `ei` is a factor).
/// Blades are always stored/read in canonical increasing-index order.
///
/// Pure integer bitmask constants — not tied to `Scalar` vs `Fixed` at
/// all — so this lives once here, shared by both
/// [`float_ga::Multivector`] and [`fixed_ga::FixedMultivector`], rather
/// than being copied into each.
pub mod blade {
    pub const S: usize = 0b0000;
    pub const E0: usize = 0b0001;
    pub const E1: usize = 0b0010;
    pub const E2: usize = 0b0100;
    pub const E3: usize = 0b1000;
    pub const E01: usize = 0b0011;
    pub const E02: usize = 0b0101;
    pub const E03: usize = 0b1001;
    pub const E12: usize = 0b0110;
    pub const E13: usize = 0b1010;
    pub const E23: usize = 0b1100;
    pub const E012: usize = 0b0111;
    pub const E013: usize = 0b1011;
    pub const E023: usize = 0b1101;
    pub const E123: usize = 0b1110;
    pub const E0123: usize = 0b1111;
}

/// A scalar type usable throughout the generic (non-GPU-dispatch-bound)
/// half of this crate — implemented for [`float_ga::Scalar`] (`f32`) and
/// [`fixed_ga::Fixed`]. Every method is a thin forward to that type's own
/// inherent method (Rust resolves an inherent method over a
/// same-named trait method, so `self.sqrt()` inside `impl ScalarLike for
/// Scalar` calls `f32::sqrt`, not itself) — this trait exists to *name*
/// the operations generic code needs, not to reimplement them.
pub trait ScalarLike:
    Copy
    + core::fmt::Debug
    + PartialEq
    + PartialOrd
    + Add<Output = Self>
    + Sub<Output = Self>
    + Mul<Output = Self>
    + Div<Output = Self>
    + Neg<Output = Self>
{
    const ZERO: Self;
    const ONE: Self;
    /// A small tolerance for degenerate (near-zero) guards — the same
    /// role `meridian_numeric_core::EPSILON` plays for `f32` and
    /// `fixed_ga`'s own internal tolerance plays for `Fixed`.
    const EPSILON: Self;

    /// For authoring constants (`0.4` for a moment-of-inertia formula,
    /// `9.81` for gravity, ...) generically. Not meant for hot per-frame
    /// paths on the `Fixed` side (see `Fixed::from_num`'s own doc
    /// comment).
    fn from_f64(v: f64) -> Self;
    fn sqrt(self) -> Self;
    fn tan(self) -> Self;
    fn sin_cos(self) -> (Self, Self);
    fn signum(self) -> Self;
    fn max(self, other: Self) -> Self;
}

/// A 3D vector type usable generically — implemented for
/// [`float_ga::Vec3`] and [`fixed_ga::FixedVec3`]. See [`ScalarLike`]'s
/// doc comment for why every method is a thin forward, not a
/// reimplementation.
pub trait VectorLike:
    Copy
    + core::fmt::Debug
    + PartialEq
    + Add<Output = Self>
    + Sub<Output = Self>
    + Neg<Output = Self>
    + Mul<<Self as VectorLike>::Scalar, Output = Self>
{
    type Scalar: ScalarLike;

    const ZERO: Self;

    fn new(x: Self::Scalar, y: Self::Scalar, z: Self::Scalar) -> Self;
    fn x(self) -> Self::Scalar;
    fn y(self) -> Self::Scalar;
    fn z(self) -> Self::Scalar;
    fn dot(self, rhs: Self) -> Self::Scalar;
    fn length(self) -> Self::Scalar;
    fn length_squared(self) -> Self::Scalar;
    fn normalize(self) -> Self;
}

/// A rotation-generator bivector type usable generically —
/// implemented for [`float_ga::Bivector3`] and
/// [`fixed_ga::FixedBivector3`].
pub trait BivectorLike:
    Copy + core::fmt::Debug + Add<Output = Self> + Sub<Output = Self> + Mul<<Self as BivectorLike>::Scalar, Output = Self>
{
    type Scalar: ScalarLike;
    type Vector: VectorLike<Scalar = Self::Scalar>;
    type Rotor: RotorLike<Scalar = Self::Scalar, Vector = Self::Vector>;

    const ZERO: Self;

    fn wedge(a: Self::Vector, b: Self::Vector) -> Self;
    fn length(self) -> Self::Scalar;
    fn exp(self) -> Self::Rotor;
}

/// A pure-rotation type usable generically — implemented for
/// [`float_ga::Rotor`] and [`fixed_ga::FixedRotor`].
pub trait RotorLike: Copy + core::fmt::Debug {
    type Scalar: ScalarLike;
    type Vector: VectorLike<Scalar = Self::Scalar>;

    fn identity() -> Self;
    fn from_axis_angle(axis: Self::Vector, angle: Self::Scalar) -> Self;
    fn compose(self, rhs: Self) -> Self;
    fn reverse(self) -> Self;
    fn transform_vector(self, v: Self::Vector) -> Self::Vector;
}

/// A rigid-motion (rotation + translation) type usable generically —
/// implemented for [`float_ga::Motor3`] and [`fixed_ga::FixedMotor3`].
pub trait MotorLike: Copy + core::fmt::Debug {
    type Scalar: ScalarLike;
    type Vector: VectorLike<Scalar = Self::Scalar>;
    type Rotor: RotorLike<Scalar = Self::Scalar, Vector = Self::Vector>;

    fn identity() -> Self;
    fn translation(t: Self::Vector) -> Self;
    fn from_rotation_translation(rotor: Self::Rotor, t: Self::Vector) -> Self;
    fn compose(self, rhs: Self) -> Self;
    fn inverse(self) -> Self;
    fn transform_point(self, p: Self::Vector) -> Self::Vector;
    fn transform_vector(self, v: Self::Vector) -> Self::Vector;
}

/// One complete "flavor" of the algebra — the associated-type bundle
/// that lets generic code (this crate's own primitives below,
/// `physics-core`'s engine, ...) be written once against `F: GaFlavor`
/// instead of once per scalar type. [`float_ga::FloatFlavor`] and
/// [`fixed_ga::FixedFlavor`] are the two flavors that exist today; a
/// third is exactly as easy to add as implementing this trait (plus the
/// five component traits above) for a new scalar/vector/bivector/rotor/
/// motor set.
pub trait GaFlavor: Copy + Default + core::fmt::Debug + PartialEq + 'static {
    type Scalar: ScalarLike;
    type Vector: VectorLike<Scalar = Self::Scalar>;
    type Bivector: BivectorLike<Scalar = Self::Scalar, Vector = Self::Vector, Rotor = Self::Rotor>;
    type Rotor: RotorLike<Scalar = Self::Scalar, Vector = Self::Vector>;
    type Motor: MotorLike<Scalar = Self::Scalar, Vector = Self::Vector, Rotor = Self::Rotor>;
}

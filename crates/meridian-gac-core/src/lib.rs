//! Geometric Algebra Core — vectors, bivectors, multivectors, rotors and motors; the shared spatial math foundation for every other subsystem.

use meridian_numeric_core::Scalar;

/// The general element of the algebra: closed under the geometric product.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Multivector(pub [Scalar; 16]);

/// A single k-vector term (scalar, vector, bivector, ...).
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Blade {
    pub grade: u8,
    pub value: Scalar,
}

/// A pure rotation.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rotor(pub Multivector);

/// Rotation + translation, composable via the geometric product. This is
/// what `Transform` is built from workspace-wide — see docs/gac-design.md.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Motor3(pub Multivector);

/// A named reference frame: origin + basis, expressed as a motor.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Frame {
    pub motor: Motor3,
}

/// A camera/projective mapping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projection(pub [[Scalar; 4]; 4]);

impl Default for Projection {
    fn default() -> Self {
        let mut m = [[0.0; 4]; 4];
        for (i, row) in m.iter_mut().enumerate() {
            row[i] = 1.0;
        }
        Self(m)
    }
}

//! Camera projection matrices, generic over the flavor.

use crate::flavor::{GaFlavor, ScalarLike};

/// A camera/projective mapping: view-space (right-handed, looking down
/// `-Z`, `+X` right, `+Y` up) to clip space. Column-major, column-vector
/// convention (`M * v`), depth range `[0, 1]` — matches wgpu/DX12/Metal,
/// not classic OpenGL's `[-1, 1]`. This is the one hardcoded convention
/// choice here: `rendering-domain`'s `Camera` is responsible for turning a
/// `Motor3`/`FixedMotor3` world frame (whose local-forward axis is a
/// per-subsystem convention, not a `gac` one) into this view space
/// before applying a `Projection` — see docs/graphics-design.md. Generic
/// like every other primitive here — most consumers will only ever build
/// one with `F = FloatFlavor` (a GPU pipeline needs `f32` uniforms
/// regardless of which flavor computed the camera's pose, via
/// `to_float_lossy`), but nothing stops a caller from deriving one
/// deterministically in `Fixed` first if that's useful.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projection<F: GaFlavor>(pub [[F::Scalar; 4]; 4]);

impl<F: GaFlavor> Default for Projection<F> {
    fn default() -> Self {
        let mut m = [[F::Scalar::ZERO; 4]; 4];
        for (i, row) in m.iter_mut().enumerate() {
            row[i] = F::Scalar::ONE;
        }
        Self(m)
    }
}

impl<F: GaFlavor> Projection<F> {
    /// A right-handed perspective projection. `fov_y_radians` is the full
    /// vertical field of view; `aspect` is width/height; `near`/`far` are
    /// positive view-space distances (`0 < near < far`).
    ///
    /// Derivation (standard perspective projection, re-derived here rather
    /// than copied from a specific library so it can be cross-checked
    /// against hand-computed values in tests): a point `(x, y, z)` in view
    /// space (`z` negative in front of the camera) projects to
    /// `x' = x * f / aspect`, `y' = y * f`, where `f = cot(fov_y / 2)`; the
    /// clip-space `w` is `-z`; depth is mapped to `[0, 1]` via
    /// `z' = far * (z + near) / (z * (far - near))` after the perspective
    /// divide, which requires `z_row = [0, 0, far / (near - far), -1]` and
    /// `w_row = [0, 0, near * far / (near - far), 0]` in the matrix below.
    pub fn perspective(
        fov_y_radians: F::Scalar,
        aspect: F::Scalar,
        near: F::Scalar,
        far: F::Scalar,
    ) -> Self {
        let zero = F::Scalar::ZERO;
        let one = F::Scalar::ONE;
        let two = one + one;
        let f = one / (fov_y_radians / two).tan();
        Self([
            [f / aspect, zero, zero, zero],
            [zero, f, zero, zero],
            [zero, zero, far / (near - far), -one],
            [zero, zero, near * far / (near - far), zero],
        ])
    }

    /// A right-handed orthographic projection over the view-space box
    /// `[left, right] x [bottom, top] x [-far, -near]`, depth mapped to
    /// `[0, 1]`.
    pub fn orthographic(
        left: F::Scalar,
        right: F::Scalar,
        bottom: F::Scalar,
        top: F::Scalar,
        near: F::Scalar,
        far: F::Scalar,
    ) -> Self {
        let zero = F::Scalar::ZERO;
        let one = F::Scalar::ONE;
        let two = one + one;
        Self([
            [two / (right - left), zero, zero, zero],
            [zero, two / (top - bottom), zero, zero],
            [zero, zero, -one / (far - near), zero],
            [
                -(right + left) / (right - left),
                -(top + bottom) / (top - bottom),
                -near / (far - near),
                one,
            ],
        ])
    }
}

//! A joint's local transform: translation, rotation, scale.

use meridian_gac::{Motor3, Rotor, Vec3};

/// A column-major 4 x 4 matrix, `m[column][row]`, the layout `gac`'s
/// `Motor3::to_mat4` and glTF both use.
pub type JointMatrix = [[f32; 4]; 4];

/// Where a joint sits relative to its parent: scaled, then rotated, then
/// moved. Scale may differ per axis, which is why a joint is not a `Motor3`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointTransform {
    /// Offset from the parent, in the parent's space.
    pub translation: Vec3,
    /// Orientation relative to the parent.
    pub rotation: Rotor,
    /// Per-axis scale, applied before the rotation.
    pub scale: Vec3,
}

impl Default for JointTransform {
    fn default() -> Self {
        Self {
            translation: Vec3::ZERO,
            rotation: Rotor::identity(),
            scale: Vec3::new(1.0, 1.0, 1.0),
        }
    }
}

impl JointTransform {
    /// The transform `t` of the way from `self` to `other`: translation and
    /// scale blend linearly, rotation along the shorter arc.
    #[must_use]
    pub fn blend(self, other: JointTransform, t: f32) -> JointTransform {
        let lerp = |a: Vec3, b: Vec3| a + (b - a) * t;
        JointTransform {
            translation: lerp(self.translation, other.translation),
            rotation: self.rotation.slerp(other.rotation, t),
            scale: lerp(self.scale, other.scale),
        }
    }

    /// The transform as a matrix, `translation * rotation * scale`.
    #[must_use]
    pub fn to_matrix(self) -> JointMatrix {
        let mut matrix =
            Motor3::from_rotation_translation(self.rotation, self.translation).to_mat4();
        let scale = [self.scale.x, self.scale.y, self.scale.z];
        for (column, factor) in matrix.iter_mut().zip(scale) {
            for component in column.iter_mut().take(3) {
                *component *= factor;
            }
        }
        matrix
    }
}

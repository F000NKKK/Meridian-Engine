//! The part of a perspective camera's view volume a shadow cascade covers.

use meridian_gac::Vec3;

use crate::Camera;

/// A perspective camera's view volume, read back out of its projection matrix.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ViewVolume {
    /// Distance of the near plane along the view direction.
    pub(super) near: f32,
    /// Distance of the far plane along the view direction.
    pub(super) far: f32,
    /// Half-width over depth at any distance.
    tan_x: f32,
    /// Half-height over depth at any distance.
    tan_y: f32,
}

impl ViewVolume {
    /// The volume of `camera`, or `None` when its projection is not a
    /// perspective one (an orthographic or identity projection has no
    /// frustum to fit cascades to).
    pub(super) fn of(camera: &Camera) -> Option<Self> {
        let m = camera.projection.0;
        let perspective =
            (m[2][3] + 1.0).abs() < 1e-6 && m[3][3].abs() < 1e-6 && m[0][0] > 0.0 && m[1][1] > 0.0;
        if !perspective || m[2][2] >= 0.0 {
            return None;
        }
        let near = m[3][2] / m[2][2];
        let far = m[2][2] * near / (1.0 + m[2][2]);
        (near > 0.0 && far > near).then_some(Self {
            near,
            far,
            tan_x: 1.0 / m[0][0],
            tan_y: 1.0 / m[1][1],
        })
    }

    /// The eight world-space corners of the slice between the view distances
    /// `from` and `to`.
    pub(super) fn slice_corners(self, camera: &Camera, from: f32, to: f32) -> [Vec3; 8] {
        let mut corners = [Vec3::ZERO; 8];
        for (index, corner) in corners.iter_mut().enumerate() {
            let depth = if index < 4 { from } else { to };
            let x = if index & 1 == 0 { -1.0 } else { 1.0 } * depth * self.tan_x;
            let y = if index & 2 == 0 { -1.0 } else { 1.0 } * depth * self.tan_y;
            // View space looks down -Z with +X right; the camera frame's
            // local axes are forward +X, up +Y, right +Z (see `Camera`).
            *corner = camera.frame.transform_point(Vec3::new(depth, y, x));
        }
        corners
    }
}

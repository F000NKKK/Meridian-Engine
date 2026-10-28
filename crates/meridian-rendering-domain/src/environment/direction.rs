//! The equirectangular mapping between directions and image coordinates.

use core::f32::consts::{FRAC_1_PI, PI, TAU};

/// The direction `(x, y, z)`, `y` up, that image coordinate `(u, v)` looks at:
/// `u = 0.5` is straight ahead along `-z`, increasing `u` turns towards `+x`;
/// `v = 0` is straight up. The environment shader uses the inverse of this.
#[must_use]
pub fn uv_to_direction(u: f32, v: f32) -> [f32; 3] {
    let azimuth = (u - 0.5) * TAU;
    let latitude = (0.5 - v) * PI;
    let flat = latitude.cos();
    [azimuth.sin() * flat, latitude.sin(), -azimuth.cos() * flat]
}

/// The image coordinate `(u, v)` of unit direction `direction`: the inverse of
/// [`uv_to_direction`].
#[must_use]
pub fn direction_to_uv(direction: [f32; 3]) -> [f32; 2] {
    let [x, y, z] = direction;
    [
        0.5 + x.atan2(-z) / TAU,
        0.5 - y.clamp(-1.0, 1.0).asin() * FRAC_1_PI,
    ]
}

//! The blending a keyframed value needs.

use meridian_gac::Rotor;

/// `a` at `t = 0`, `b` at `t = 1`, component by component.
pub(super) fn lerp<const N: usize>(a: [f32; N], b: [f32; N], t: f32) -> [f32; N] {
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * t)
}

/// The Hermite spline through `[p0, m0, p1, m1]`: from `p0` (leaving with
/// tangent `m0`) to `p1` (arriving with tangent `m1`), `t` of the way along a
/// segment `dt` long.
pub(super) fn hermite<const N: usize>(
    [p0, m0, p1, m1]: [[f32; N]; 4],
    t: f32,
    dt: f32,
) -> [f32; N] {
    let (t2, t3) = (t * t, t * t * t);
    let (h00, h10) = (2.0 * t3 - 3.0 * t2 + 1.0, t3 - 2.0 * t2 + t);
    let (h01, h11) = (-2.0 * t3 + 3.0 * t2, t3 - t2);
    std::array::from_fn(|i| h00 * p0[i] + h10 * dt * m0[i] + h01 * p1[i] + h11 * dt * m1[i])
}

/// The quaternion `a` turned `t` of the way to `b` along the shorter arc.
pub(super) fn slerp(a: [f32; 4], b: [f32; 4], t: f32) -> [f32; 4] {
    Rotor::from_quaternion(a)
        .slerp(Rotor::from_quaternion(b), t)
        .to_quaternion()
}

/// `quaternion` scaled to unit length.
pub(super) fn normalised(quaternion: [f32; 4]) -> [f32; 4] {
    Rotor::from_quaternion(quaternion).to_quaternion()
}

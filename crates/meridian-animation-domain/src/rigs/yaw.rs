//! The heading (rotation about the vertical axis) of a joint's rotation.

use std::f32::consts::{PI, TAU};

use meridian_gac::Rotor;

type Quaternion = [f32; 4];

/// `a * b` with `[x, y, z, w]` quaternions: `b` acts first.
fn multiply(a: Quaternion, b: Quaternion) -> Quaternion {
    [
        a[3] * b[0] + a[0] * b[3] + a[1] * b[2] - a[2] * b[1],
        a[3] * b[1] - a[0] * b[2] + a[1] * b[3] + a[2] * b[0],
        a[3] * b[2] + a[0] * b[1] - a[1] * b[0] + a[2] * b[3],
        a[3] * b[3] - a[0] * b[0] - a[1] * b[1] - a[2] * b[2],
    ]
}

fn conjugate(q: Quaternion) -> Quaternion {
    [-q[0], -q[1], -q[2], q[3]]
}

/// The part of `q` that turns about y, as a unit quaternion; the identity
/// when the rotation is a half turn about another axis and has none.
fn heading(q: Quaternion) -> Quaternion {
    let length = q[1].hypot(q[3]);
    if length <= f32::EPSILON {
        [0.0, 0.0, 0.0, 1.0]
    } else {
        [0.0, q[1] / length, 0.0, q[3] / length]
    }
}

/// How far `rotation` turns about y, in radians. Only differences of this
/// are meaningful: the value is ambiguous by whole turns.
pub(super) fn yaw_of(rotation: Rotor) -> f32 {
    let q = heading(rotation.to_quaternion());
    2.0 * q[1].atan2(q[3])
}

/// `rotation` with its turn about y replaced by the one of `rest`, the tilt
/// and roll kept.
pub(super) fn with_yaw_of(rotation: Rotor, rest: Rotor) -> Rotor {
    let q = rotation.to_quaternion();
    let tilt = multiply(conjugate(heading(q)), q);
    Rotor::from_quaternion(multiply(heading(rest.to_quaternion()), tilt))
}

/// `angle` as an equivalent one in `(-pi, pi]`.
pub(super) fn wrapped(angle: f32) -> f32 {
    let turned = angle.rem_euclid(TAU);
    if turned > PI { turned - TAU } else { turned }
}

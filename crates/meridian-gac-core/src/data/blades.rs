//! The stored components of even multivectors, in `gac-core`'s conventions:
//! a rotor as `[s, e23, e31, e12]` (the multivector keeps `e13`, so `e31`
//! is its exact negation), a motor as the rotor's four then
//! `[e01, e02, e03, e0123]`. The other blades are zero by construction and
//! are not stored.

use crate::blade;
use crate::flavor::ScalarLike;

/// A rotor's stored components `[s, e23, e31, e12]`.
pub(crate) fn rotor_components<S: ScalarLike>(m: &[S; 16]) -> [S; 4] {
    [m[blade::S], m[blade::E23], -m[blade::E13], m[blade::E12]]
}

/// The multivector of a rotor's stored components.
pub(crate) fn rotor_multivector<S: ScalarLike>([s, e23, e31, e12]: [S; 4]) -> [S; 16] {
    let mut m = [S::ZERO; 16];
    m[blade::S] = s;
    m[blade::E23] = e23;
    m[blade::E13] = -e31;
    m[blade::E12] = e12;
    m
}

/// A motor's eight even components: the rotor's, then `[e01, e02, e03, e0123]`.
pub(crate) fn motor_components<S: ScalarLike>(m: &[S; 16]) -> [S; 8] {
    let [s, e23, e31, e12] = rotor_components(m);
    let [e01, e02, e03, e0123] = [m[blade::E01], m[blade::E02], m[blade::E03], m[blade::E0123]];
    [s, e23, e31, e12, e01, e02, e03, e0123]
}

/// The multivector of a motor's eight even components.
pub(crate) fn motor_multivector<S: ScalarLike>(c: [S; 8]) -> [S; 16] {
    let mut m = rotor_multivector([c[0], c[1], c[2], c[3]]);
    m[blade::E01] = c[4];
    m[blade::E02] = c[5];
    m[blade::E03] = c[6];
    m[blade::E0123] = c[7];
    m
}

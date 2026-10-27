//! The fixed flavor's stored forms: how `Fixed` and [`crate::fixed_ga`]'s
//! types map to their components. The schema logic itself is generic, in
//! [`super`].

use meridian_numeric_core::Fixed;

use super::{
    DataScalar, Stored, motor_multivector, motor_parts, motor_schema, rotor_multivector,
    rotor_parts, stored_schema,
};
use crate::fixed_ga::{FixedBivector3, FixedMotor3, FixedMultivector, FixedRotor, FixedVec3};

impl DataScalar for Fixed {
    fn same_bits(self, other: Self) -> bool {
        self == other
    }
}

impl Stored<3> for FixedVec3 {
    type S = Fixed;
    const NAME: &'static str = "fixed_vec3";
    fn parts(&self) -> [Fixed; 3] {
        [self.x, self.y, self.z]
    }
    fn from_parts([x, y, z]: [Fixed; 3]) -> Self {
        FixedVec3::new(x, y, z)
    }
}

impl Stored<3> for FixedBivector3 {
    type S = Fixed;
    const NAME: &'static str = "fixed_bivector3";
    fn parts(&self) -> [Fixed; 3] {
        [self.e23, self.e31, self.e12]
    }
    fn from_parts([e23, e31, e12]: [Fixed; 3]) -> Self {
        FixedBivector3::new(e23, e31, e12)
    }
}

impl Stored<4> for FixedRotor {
    type S = Fixed;
    const NAME: &'static str = "fixed_rotor";
    fn parts(&self) -> [Fixed; 4] {
        rotor_parts(&self.0.0)
    }
    fn from_parts(parts: [Fixed; 4]) -> Self {
        FixedRotor(FixedMultivector(rotor_multivector(parts)))
    }
}

impl Stored<8> for FixedMotor3 {
    type S = Fixed;
    const NAME: &'static str = "fixed_motor";
    fn parts(&self) -> [Fixed; 8] {
        motor_parts(&self.0.0)
    }
    fn from_parts(parts: [Fixed; 8]) -> Self {
        FixedMotor3(FixedMultivector(motor_multivector(parts)))
    }
}

stored_schema!(FixedVec3 => 3, FixedBivector3 => 3, FixedRotor => 4);
motor_schema!(FixedMotor3);

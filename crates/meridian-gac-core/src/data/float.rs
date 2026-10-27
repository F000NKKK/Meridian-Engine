//! The float flavor's stored forms: how `f32` and [`crate::float_ga`]'s
//! types map to their components. The schema logic itself is generic, in
//! [`super`].

use meridian_numeric_core::Scalar;

use super::{
    DataScalar, Stored, motor_multivector, motor_parts, motor_schema, rotor_multivector,
    rotor_parts, stored_schema,
};
use crate::float_ga::{Bivector3, Motor3, Multivector, Rotor, Vec3};

impl DataScalar for Scalar {
    fn same_bits(self, other: Self) -> bool {
        self.to_bits() == other.to_bits()
    }
}

impl Stored<3> for Vec3 {
    type S = Scalar;
    const NAME: &'static str = "vec3";
    fn parts(&self) -> [Scalar; 3] {
        [self.x, self.y, self.z]
    }
    fn from_parts([x, y, z]: [Scalar; 3]) -> Self {
        Vec3::new(x, y, z)
    }
}

impl Stored<3> for Bivector3 {
    type S = Scalar;
    const NAME: &'static str = "bivector3";
    fn parts(&self) -> [Scalar; 3] {
        [self.e23, self.e31, self.e12]
    }
    fn from_parts([e23, e31, e12]: [Scalar; 3]) -> Self {
        Bivector3::new(e23, e31, e12)
    }
}

impl Stored<4> for Rotor {
    type S = Scalar;
    const NAME: &'static str = "rotor";
    fn parts(&self) -> [Scalar; 4] {
        rotor_parts(&self.0.0)
    }
    fn from_parts(parts: [Scalar; 4]) -> Self {
        Rotor(Multivector(rotor_multivector(parts)))
    }
}

impl Stored<8> for Motor3 {
    type S = Scalar;
    const NAME: &'static str = "motor";
    fn parts(&self) -> [Scalar; 8] {
        motor_parts(&self.0.0)
    }
    fn from_parts(parts: [Scalar; 8]) -> Self {
        Motor3(Multivector(motor_multivector(parts)))
    }
}

stored_schema!(Vec3 => 3, Bivector3 => 3, Rotor => 4);
motor_schema!(Motor3);

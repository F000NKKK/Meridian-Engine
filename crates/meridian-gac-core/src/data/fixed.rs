//! The fixed flavour: `Fixed` and [`crate::fixed_ga`]'s types bound to the
//! generic stored forms (components are raw `Fixed` bits).

use meridian_numeric_core::Fixed;
use substrate_data::named_schema;
use substrate_data::schema::NamedType;

use super::blades::{motor_components, motor_multivector, rotor_components, rotor_multivector};
use super::{MotorBlades, MotorForm, StoredScalar};
use crate::fixed_ga::{FixedBivector3, FixedMotor3, FixedMultivector, FixedRotor, FixedVec3};

impl StoredScalar for Fixed {
    fn same_bits(self, other: Self) -> bool {
        self == other
    }
}

impl NamedType for FixedVec3 {
    const NAME: &'static str = "fixed_vec3";
    type Stored = [Fixed; 3];
    fn to_stored(&self) -> [Fixed; 3] {
        [self.x, self.y, self.z]
    }
    fn from_stored([x, y, z]: [Fixed; 3]) -> Option<Self> {
        Some(FixedVec3::new(x, y, z))
    }
}

impl NamedType for FixedBivector3 {
    const NAME: &'static str = "fixed_bivector3";
    type Stored = [Fixed; 3];
    fn to_stored(&self) -> [Fixed; 3] {
        [self.e23, self.e31, self.e12]
    }
    fn from_stored([e23, e31, e12]: [Fixed; 3]) -> Option<Self> {
        Some(FixedBivector3::new(e23, e31, e12))
    }
}

impl NamedType for FixedRotor {
    const NAME: &'static str = "fixed_rotor";
    type Stored = [Fixed; 4];
    fn to_stored(&self) -> [Fixed; 4] {
        rotor_components(&self.0.0)
    }
    fn from_stored(components: [Fixed; 4]) -> Option<Self> {
        Some(FixedRotor(FixedMultivector(rotor_multivector(components))))
    }
}

impl MotorBlades for FixedMotor3 {
    fn even_components(&self) -> [Fixed; 8] {
        motor_components(&self.0.0)
    }
    fn from_even_components(components: [Fixed; 8]) -> Self {
        FixedMotor3(FixedMultivector(motor_multivector(components)))
    }
}

impl NamedType for FixedMotor3 {
    const NAME: &'static str = "fixed_motor";
    type Stored = MotorForm<Fixed>;
    fn to_stored(&self) -> MotorForm<Fixed> {
        self.form()
    }
    fn from_stored(form: MotorForm<Fixed>) -> Option<Self> {
        Self::from_form(form)
    }
}

named_schema!(FixedVec3);
named_schema!(FixedBivector3);
named_schema!(FixedRotor);
named_schema!(FixedMotor3);

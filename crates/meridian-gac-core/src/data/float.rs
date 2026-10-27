//! The float flavour: `f32` and [`crate::float_ga`]'s types bound to the
//! generic stored forms.

use meridian_numeric_core::Scalar;
use substrate_data::named_schema;
use substrate_data::schema::NamedType;

use super::blades::{motor_components, motor_multivector, rotor_components, rotor_multivector};
use super::{MotorBlades, MotorForm, StoredScalar};
use crate::float_ga::{Bivector3, Motor3, Multivector, Rotor, Vec3};

impl StoredScalar for Scalar {
    fn same_bits(self, other: Self) -> bool {
        self.to_bits() == other.to_bits()
    }
}

impl NamedType for Vec3 {
    const NAME: &'static str = "vec3";
    type Stored = [Scalar; 3];
    fn to_stored(&self) -> [Scalar; 3] {
        [self.x, self.y, self.z]
    }
    fn from_stored([x, y, z]: [Scalar; 3]) -> Option<Self> {
        Some(Vec3::new(x, y, z))
    }
}

impl NamedType for Bivector3 {
    const NAME: &'static str = "bivector3";
    type Stored = [Scalar; 3];
    fn to_stored(&self) -> [Scalar; 3] {
        [self.e23, self.e31, self.e12]
    }
    fn from_stored([e23, e31, e12]: [Scalar; 3]) -> Option<Self> {
        Some(Bivector3::new(e23, e31, e12))
    }
}

impl NamedType for Rotor {
    const NAME: &'static str = "rotor";
    type Stored = [Scalar; 4];
    fn to_stored(&self) -> [Scalar; 4] {
        rotor_components(&self.0.0)
    }
    fn from_stored(components: [Scalar; 4]) -> Option<Self> {
        Some(Rotor(Multivector(rotor_multivector(components))))
    }
}

impl MotorBlades for Motor3 {
    fn even_components(&self) -> [Scalar; 8] {
        motor_components(&self.0.0)
    }
    fn from_even_components(components: [Scalar; 8]) -> Self {
        Motor3(Multivector(motor_multivector(components)))
    }
}

impl NamedType for Motor3 {
    const NAME: &'static str = "motor";
    type Stored = MotorForm<Scalar>;
    fn to_stored(&self) -> MotorForm<Scalar> {
        self.form()
    }
    fn from_stored(form: MotorForm<Scalar>) -> Option<Self> {
        Self::from_form(form)
    }
}

named_schema!(Vec3);
named_schema!(Bivector3);
named_schema!(Rotor);
named_schema!(Motor3);

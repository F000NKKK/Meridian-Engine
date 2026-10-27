//! [`MotorBlades`]: a motor type and its stored form.

use substrate_data::schema::NamedType;

use super::{MotorForm, StoredScalar};
use crate::flavor::{MotorLike, ScalarLike, VectorLike};

/// A motor type whose rotor and translation vector are named types stored
/// as component arrays, with its eight even components exposed — enough to
/// convert it to and from its [`MotorForm`] once, for both flavours.
pub(crate) trait MotorBlades: MotorLike<Scalar: StoredScalar>
where
    Self::Rotor: NamedType<Stored = [Self::Scalar; 4]>,
    Self::Vector: NamedType<Stored = [Self::Scalar; 3]>,
{
    /// The rotor's components, then `[e01, e02, e03, e0123]`.
    fn even_components(&self) -> [Self::Scalar; 8];

    /// The motor of those components.
    fn from_even_components(components: [Self::Scalar; 8]) -> Self;

    /// Its form: `translation` when that rebuilds it bit-exactly, else `dual`.
    fn form(&self) -> MotorForm<Self::Scalar> {
        let c = self.even_components();
        let rotor = [c[0], c[1], c[2], c[3]];
        let t = self.transform_point(<Self::Vector as VectorLike>::ZERO);
        let rebuilt = Self::Rotor::from_stored(rotor)
            .map(|r| Self::from_rotation_translation(r, t).even_components());
        let exact = rebuilt.is_some_and(|r| Self::Scalar::all_same_bits(r, c));
        MotorForm {
            rotor,
            translation: exact.then(|| t.to_stored()),
            dual: (!exact).then_some([c[4], c[5], c[6], c[7]]),
        }
    }

    /// The motor `form` describes; `None` when it has both `translation` and `dual`.
    fn from_form(form: MotorForm<Self::Scalar>) -> Option<Self> {
        let r = form.rotor;
        match (form.translation, form.dual) {
            (Some(_), Some(_)) => None,
            (_, Some(d)) => Some(Self::from_even_components([
                r[0], r[1], r[2], r[3], d[0], d[1], d[2], d[3],
            ])),
            (t, None) => {
                let t = Self::Vector::from_stored(t.unwrap_or([Self::Scalar::ZERO; 3]))?;
                Some(Self::from_rotation_translation(
                    Self::Rotor::from_stored(r)?,
                    t,
                ))
            }
        }
    }
}

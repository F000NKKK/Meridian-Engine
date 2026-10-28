//! [`LightForm`]: `rendering-domain`'s `LightSource` as files hold it.

use meridian_rendering_domain::LightSource;
use substrate_data::Schema;

use super::LightKindForm;

/// A light component: its kind, colour and intensity. A newtype of this crate
/// because `rendering-domain` stays file-blind.
#[derive(Debug, Clone, Copy, PartialEq, Schema)]
pub struct LightForm {
    /// How it spreads.
    pub kind: LightKindForm,
    /// Linear RGB; white when left out.
    #[data(default = "white")]
    pub color: [f32; 3],
    /// Scales `color`; `1` when left out.
    #[data(default = "one")]
    pub intensity: f32,
}

fn white() -> [f32; 3] {
    [1.0; 3]
}

fn one() -> f32 {
    1.0
}

impl From<LightSource> for LightForm {
    fn from(light: LightSource) -> Self {
        Self {
            kind: light.kind.into(),
            color: light.color,
            intensity: light.intensity,
        }
    }
}

impl From<LightForm> for LightSource {
    fn from(form: LightForm) -> Self {
        Self {
            kind: form.kind.into(),
            color: form.color,
            intensity: form.intensity,
        }
    }
}

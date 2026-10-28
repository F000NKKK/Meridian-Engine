//! [`LightCodec`]: registers `LightSource` through [`LightForm`](super::LightForm).

use meridian_rendering_domain::LightSource;
use substrate_data::with::With;
use substrate_data::{Reader, Schema, Shape, Value, ValueRef};

use super::LightForm;

/// `registry.register_with::<LightSource, LightCodec>(name)`.
#[derive(Debug, Clone, Copy, Default)]
pub struct LightCodec;

impl With<LightSource> for LightCodec {
    fn shape() -> Shape {
        LightForm::shape()
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<LightSource> {
        LightForm::read(v, cx).map(LightSource::from)
    }
    fn write(light: &LightSource) -> Value {
        LightForm::from(*light).write()
    }
}

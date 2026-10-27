//! `ecs-core::Transform` in files, through [`TransformData`] — a newtype
//! of this crate implementing `Schema` and converting to and from
//! `Transform` — since `ecs-core` stays file-blind (no `data` feature, a
//! team/core decision) and the orphan rule keeps `Schema` off `Transform`
//! anywhere else.

use meridian_ecs_core::Transform;
use meridian_gac_core::Motor3;
use substrate_data::schema::{Field, StructShape};
use substrate_data::with::With;
use substrate_data::{Map, Reader, Schema, Shape, Value, ValueRef};

const MOTOR: &str = "motor";

/// [`Transform`] as files hold it: `{ motor: <motor> }`, the motor in
/// `gac-core`'s stored form (`{ rotor, translation }`), identity when left
/// out.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct TransformData(pub Motor3);

impl From<Transform> for TransformData {
    fn from(t: Transform) -> Self {
        TransformData(t.motor)
    }
}

impl From<TransformData> for Transform {
    fn from(t: TransformData) -> Self {
        Transform { motor: t.0 }
    }
}

impl Schema for TransformData {
    fn shape() -> Shape {
        Shape::Struct(StructShape {
            name: "transform",
            doc: "Where an entity is: rotation, then translation, as one motor.",
            fields: vec![Field {
                name: MOTOR,
                doc: "The rigid motion from the entity's frame to its parent's. Default: identity.",
                shape: Motor3::shape,
                required: false,
                range: None,
            }],
        })
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader) -> Option<Self> {
        let map = cx.map(v)?;
        cx.own_keys(|cx| cx.unknown_keys(map, &[MOTOR]));
        cx.field_or(map, MOTOR, Motor3::identity).map(TransformData)
    }
    fn write(&self) -> Value {
        let mut m = Map::new();
        m.insert(MOTOR, self.0.write());
        Value::Map(m)
    }
}

/// Registers [`Transform`] through [`TransformData`]:
/// `registry.register_with::<Transform, TransformCodec>("transform")`.
pub struct TransformCodec;

impl With<Transform> for TransformCodec {
    fn shape() -> Shape {
        TransformData::shape()
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader) -> Option<Transform> {
        TransformData::read(v, cx).map(Transform::from)
    }
    fn write(t: &Transform) -> Value {
        TransformData::from(*t).write()
    }
}

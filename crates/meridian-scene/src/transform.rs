//! `ecs-core::Transform`'s schema, as a codec (see the crate doc for why it
//! isn't an `impl Schema`).

use meridian_ecs_core::Transform;
use meridian_gac_core::Motor3;
use substrate_data::schema::{Field, StructShape};
use substrate_data::with::With;
use substrate_data::{Map, Reader, Schema, Shape, Value, ValueRef};

const MOTOR: &str = "motor";

/// Reads and writes [`Transform`] as `{ motor: <motor> }` — the motor in
/// `gac-core`'s stored form (`{ rotor, translation }`), identity when
/// left out. Register it with
/// `registry.register_with::<Transform, TransformCodec>("transform")`.
pub struct TransformCodec;

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

impl With<Transform> for TransformCodec {
    fn shape() -> Shape {
        shape()
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader) -> Option<Transform> {
        let map = cx.map(v)?;
        cx.own_keys(|cx| cx.unknown_keys(map, &[MOTOR]));
        let motor = cx.field_or(map, MOTOR, Motor3::identity)?;
        Some(Transform { motor })
    }
    fn write(t: &Transform) -> Value {
        let mut m = Map::new();
        m.insert(MOTOR, t.motor.write());
        Value::Map(m)
    }
}

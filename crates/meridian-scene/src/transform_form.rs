//! [`TransformForm`]: `ecs-core::Transform` as files hold it.

use meridian_ecs_core::Transform;
use meridian_gac_core::Motor3;
use substrate_data::with::With;
use substrate_data::{Reader, Schema, Shape, Value, ValueRef};

/// Where an entity is: rotation, then translation, as one motor — stored
/// as `{ motor }`, identity when left out. A newtype of this crate because
/// `ecs-core` stays file-blind and the orphan rule keeps `Schema` off
/// `Transform` anywhere else.
#[derive(Debug, Clone, Copy, PartialEq, Default, Schema)]
pub struct TransformForm {
    /// The rigid motion from the entity's frame to its parent's.
    #[data(default = "Motor3::identity")]
    pub motor: Motor3,
}

impl From<Transform> for TransformForm {
    fn from(t: Transform) -> Self {
        TransformForm { motor: t.motor }
    }
}

impl From<TransformForm> for Transform {
    fn from(t: TransformForm) -> Self {
        Transform { motor: t.motor }
    }
}

/// Registers [`Transform`] through [`TransformForm`]:
/// `registry.register_with::<Transform, TransformCodec>(name)`.
#[derive(Debug, Clone, Copy, Default)]
pub struct TransformCodec;

impl With<Transform> for TransformCodec {
    fn shape() -> Shape {
        TransformForm::shape()
    }
    fn read(v: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<Transform> {
        TransformForm::read(v, cx).map(Transform::from)
    }
    fn write(t: &Transform) -> Value {
        TransformForm::from(*t).write()
    }
}

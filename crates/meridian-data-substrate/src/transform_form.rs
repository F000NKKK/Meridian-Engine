//! [`TransformForm`]: `ecs::Transform` as files hold it.

use meridian_ecs::Transform;
use meridian_gac::{Motor3, Vec3};
use substrate_data::with::With;
use substrate_data::{Reader, Schema, Shape, Value, ValueRef};

/// Where an entity is: rotation, then translation, as one motor, and how big
/// it is — stored as `{ motor, scale }`, each read as the identity motor or
/// the unit scale when left out, so files written before `scale` existed
/// load unchanged. A newtype of
/// this crate because `ecs` stays file-blind and the orphan rule keeps
/// `Schema` off `Transform` anywhere else.
#[derive(Debug, Clone, Copy, PartialEq, Schema)]
pub struct TransformForm {
    /// The rigid motion from the entity's frame to its parent's.
    #[data(default = "Motor3::identity")]
    pub motor: Motor3,
    /// The size along each local axis, applied before the motor.
    #[data(default = "unit_scale")]
    pub scale: Vec3,
}

fn unit_scale() -> Vec3 {
    Vec3::ONE
}

impl Default for TransformForm {
    fn default() -> Self {
        Transform::default().into()
    }
}

impl From<Transform> for TransformForm {
    fn from(t: Transform) -> Self {
        TransformForm {
            motor: t.motor,
            scale: t.scale,
        }
    }
}

impl From<TransformForm> for Transform {
    fn from(t: TransformForm) -> Self {
        Transform {
            motor: t.motor,
            scale: t.scale,
        }
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

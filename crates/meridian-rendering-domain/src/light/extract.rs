//! Lights out of an ECS world.

use meridian_ecs::{GlobalTransform, Transform, World};
use meridian_gac::Motor3;

use super::LightSource;
use crate::{Light, LightKind};

/// Every [`LightSource`] entity of `world` as a [`Light`] at the entity's
/// frame (its `GlobalTransform` when resolved, else its `Transform`); a light
/// without a transform sits at the origin, facing along `+X`.
#[must_use]
pub fn extract_lights(world: &World) -> Vec<Light> {
    world
        .query::<LightSource>()
        .map(|(entity, source)| {
            let frame = world
                .get::<GlobalTransform>(entity)
                .map(|global| global.motor)
                .or_else(|| world.get::<Transform>(entity).map(|local| local.motor))
                .unwrap_or_else(Motor3::identity);
            light_at(*source, frame)
        })
        .collect()
}

fn light_at(source: LightSource, frame: Motor3) -> Light {
    let LightSource {
        kind,
        color,
        intensity,
    } = source;
    match kind {
        LightKind::Directional => Light::Directional {
            direction: frame,
            color,
            intensity,
        },
        LightKind::Point { range } => Light::Point {
            position: frame,
            color,
            intensity,
            range,
        },
        LightKind::Spot {
            range,
            inner_angle,
            outer_angle,
        } => Light::Spot {
            position: frame,
            direction: frame,
            color,
            intensity,
            range,
            inner_angle,
            outer_angle,
        },
    }
}

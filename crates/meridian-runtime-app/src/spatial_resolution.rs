//! Turning independent spatial source components into the engine's rigid pose.

use meridian_ecs::{Position, Transform, World};
use meridian_gac::{Motor3, Rotor, Vec3};

use crate::{SpatialIssue, SpatialProblem};

/// Resolves Motor3 or Position/Rotors into local Transform components.
/// Existing transforms on source entities are derived outputs and are overwritten.
/// Invalid entities retain their previous transform and are returned as diagnostics.
/// Entities without sources are untouched. Run before hierarchy propagation;
/// do not combine these authorities with animation/physics writing Transform.
/// Motors must be unit rigid motions constructed through the GAC motor API.
#[must_use]
pub fn resolve_spatial_components(world: &mut World) -> Vec<SpatialIssue> {
    let resolved: Vec<_> = world
        .query_many::<(Option<&Position>, Option<&Rotor>, Option<&Motor3>)>()
        .filter(|(_, (position, rotor, motor))| {
            position.is_some() || rotor.is_some() || motor.is_some()
        })
        .map(|(entity, (position, rotor, motor))| (entity, resolve(position, rotor, motor)))
        .collect();
    let mut issues = Vec::new();
    for (entity, result) in resolved {
        match result {
            Ok(motor) => {
                if let Some(transform) = world.get_mut::<Transform>(entity) {
                    transform.motor = motor;
                } else {
                    world.insert(entity, Transform { motor });
                }
            }
            Err(problem) => issues.push(SpatialIssue { entity, problem }),
        }
    }
    issues
}

fn resolve(
    position: Option<&Position>,
    rotor: Option<&Rotor>,
    motor: Option<&Motor3>,
) -> Result<Motor3, SpatialProblem> {
    if let Some(motor) = motor {
        if position.is_some() || rotor.is_some() {
            return Err(SpatialProblem::CompetingSources);
        }
        return motor
            .0
            .0
            .iter()
            .all(|value| value.is_finite())
            .then_some(*motor)
            .ok_or(SpatialProblem::NonFinite);
    }
    let position = position.map_or(Vec3::ZERO, |position| position.0);
    if ![position.x, position.y, position.z]
        .iter()
        .all(|value| value.is_finite())
    {
        return Err(SpatialProblem::NonFinite);
    }
    let rotor = rotor
        .copied()
        .unwrap_or_default()
        .try_normalized()
        .map_err(SpatialProblem::Rotation)?;
    Ok(Motor3::from_rotation_translation(rotor, position))
}

//! Explicit ECS-to-physics synchronization using a dedicated physics world.

use meridian_ecs::{Entity, Parent, Position, Transform, World};
use meridian_gac::{Motor3, Rotor, Scale3, Vec3, blade};
use meridian_physics_app::{
    Collider, ColliderShape, PhysicsBody, PhysicsStepper, PhysicsSubsystem, RigidBody,
};

use crate::{PhysicsIssue, PhysicsProblem, resolve_spatial_components};

/// Resolves spatial sources, steps colliders and writes dynamic poses back.
/// `physics` must be dedicated to this bridge: its body list is rebuilt from ECS.
/// Root entities only; scale must be unit. Collider-only entities are static.
/// Run hierarchy propagation afterwards. Contact indices belong to this step.
/// Invalid dt rejects the entire step without changing world or physics.
///
/// # Errors
/// Returns an error for non-finite or non-positive dt before any mutation.
pub fn step_ecs_physics(
    world: &mut World,
    physics: &mut PhysicsSubsystem,
    stepper: &mut dyn PhysicsStepper,
    dt: f32,
) -> Result<Vec<PhysicsIssue>, PhysicsProblem> {
    if !dt.is_finite() || dt <= 0.0 {
        return Err(PhysicsProblem::Parameters);
    }
    let invalid = resolve_spatial_components(world);
    let mut issues: Vec<_> = invalid
        .iter()
        .map(|issue| PhysicsIssue {
            entity: issue.entity,
            problem: PhysicsProblem::Pose,
        })
        .collect();
    let mut entities: Vec<_> = world
        .query::<Collider>()
        .map(|(entity, _)| entity)
        .collect();
    entities.sort_by_key(|entity| (entity.index, entity.generation));
    let mut accepted = Vec::new();
    physics.bodies.clear();
    for entity in entities {
        if invalid.iter().any(|issue| issue.entity == entity) {
            continue;
        }
        let body = body(world, entity);
        match body {
            Ok(body) => {
                accepted.push(entity);
                physics.bodies.push(body);
            }
            Err(problem) => issues.push(PhysicsIssue { entity, problem }),
        }
    }
    stepper.step(physics, dt);
    for (entity, body) in accepted.into_iter().zip(&physics.bodies) {
        if body.mass > 0.0 {
            write_back(world, entity, body);
        }
    }
    Ok(issues)
}

fn body(world: &World, entity: Entity) -> Result<RigidBody, PhysicsProblem> {
    if world.get::<Parent>(entity).is_some()
        || world
            .get::<Scale3>(entity)
            .is_some_and(|scale| !scale.is_unit())
    {
        return Err(PhysicsProblem::ParentOrScale);
    }
    let frame = world
        .get::<Transform>(entity)
        .ok_or(PhysicsProblem::Pose)?
        .motor;
    if !frame.0.0.iter().all(|v| v.is_finite()) {
        return Err(PhysicsProblem::Pose);
    }
    let motion = world
        .get::<PhysicsBody>(entity)
        .copied()
        .unwrap_or_default();
    let shape = world
        .get::<Collider>(entity)
        .ok_or(PhysicsProblem::Parameters)?
        .0;
    let dimensions = match shape {
        ColliderShape::Sphere { radius } => [radius; 3],
        ColliderShape::Cuboid { half_extents } => [half_extents.x, half_extents.y, half_extents.z],
    };
    if !motion.mass.is_finite()
        || motion.mass < 0.0
        || !dimensions.iter().all(|v| v.is_finite() && *v > 0.0)
        || ![
            motion.velocity.x,
            motion.velocity.y,
            motion.velocity.z,
            motion.angular_velocity.e23,
            motion.angular_velocity.e31,
            motion.angular_velocity.e12,
        ]
        .iter()
        .all(|v| v.is_finite())
    {
        return Err(PhysicsProblem::Parameters);
    }
    Ok(RigidBody {
        frame,
        mass: motion.mass,
        velocity: motion.velocity,
        angular_velocity: motion.angular_velocity,
        shape,
    })
}

fn write_back(world: &mut World, entity: Entity, body: &RigidBody) {
    world.insert(entity, Transform { motor: body.frame });
    if world.get::<Motor3>(entity).is_some() {
        world.insert(entity, body.frame);
    } else if world.get::<Position>(entity).is_some() || world.get::<Rotor>(entity).is_some() {
        world.insert(entity, Position(body.frame.transform_point(Vec3::ZERO)));
        let coefficients = body.frame.0.0;
        let rotor = Rotor::from_quaternion([
            -coefficients[blade::E23],
            coefficients[blade::E13],
            -coefficients[blade::E12],
            coefficients[blade::S],
        ]);
        world.insert(entity, rotor);
    }
    if let Some(motion) = world.get_mut::<PhysicsBody>(entity) {
        motion.velocity = body.velocity;
        motion.angular_velocity = body.angular_velocity;
    }
}

//! What every physics stepper must satisfy.

use meridian_gac::{Motor3, Vec3};
use meridian_physics_domain::{ColliderShape, ConstraintSolver, RigidBody};

use crate::{PhysicsStepper, PhysicsSubsystem};

const DT: f32 = 1.0 / 60.0;

fn floor_and_box() -> PhysicsSubsystem {
    let mut physics = PhysicsSubsystem {
        solver: ConstraintSolver::new(0.0).with_friction(0.6),
        ..Default::default()
    };
    physics.bodies.push(RigidBody {
        frame: Motor3::translation(Vec3::new(0.0, -0.5, 0.0)),
        mass: 0.0,
        shape: ColliderShape::Cuboid {
            half_extents: Vec3::new(14.0, 0.5, 14.0),
        },
        ..Default::default()
    });
    physics.bodies.push(RigidBody {
        frame: Motor3::translation(Vec3::new(0.0, 3.0, 0.0)),
        mass: 1.0,
        shape: ColliderShape::Cuboid {
            half_extents: Vec3::new(0.6, 0.6, 0.6),
        },
        ..Default::default()
    });
    physics
}

/// Runs the stepper contract against `stepper`:
///
/// - a free body falls under gravity (its velocity changes in one step);
/// - a box dropped on a static floor settles without bouncing or sinking
///   (the multi-point-manifold regression: positional correction applied
///   once per tick, not once per contact point).
///
/// # Panics
///
/// Panics, naming the broken rule, when `stepper` violates the contract.
pub fn run_stepper_contract(stepper: &mut dyn PhysicsStepper) {
    let mut free = PhysicsSubsystem::default();
    free.bodies.push(RigidBody {
        frame: Motor3::translation(Vec3::new(0.0, 10.0, 0.0)),
        mass: 1.0,
        shape: ColliderShape::Sphere { radius: 0.5 },
        ..Default::default()
    });
    let before = free.bodies[0].velocity;
    stepper.step(&mut free, DT);
    assert_ne!(
        free.bodies[0].velocity, before,
        "a free body must accelerate under gravity"
    );

    let mut physics = floor_and_box();
    let mut lowest = f32::MAX;
    let mut highest = f32::MIN;
    for step in 0..600 {
        stepper.step(&mut physics, DT);
        if step > 200 {
            let height = physics.bodies[1].position().y;
            lowest = lowest.min(height);
            highest = highest.max(height);
        }
    }
    assert!(
        highest - lowest < 0.01,
        "a settled box must not bounce (min {lowest}, max {highest})"
    );
    assert!(
        lowest > 0.0,
        "a settled box must not clip through the floor (min {lowest})"
    );
}

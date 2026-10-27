//! Shared constructors for the float-flavor physics tests.

use meridian_gac::{Motor3, Vec3};

use crate::float::*;

pub(super) fn sphere(position: Vec3, velocity: Vec3, mass: f32, radius: f32) -> RigidBody {
    RigidBody {
        frame: Motor3::translation(position),
        velocity,
        mass,
        shape: ColliderShape::Sphere { radius },
        ..Default::default()
    }
}

pub(super) fn cuboid(frame: Motor3, velocity: Vec3, mass: f32, half_extents: Vec3) -> RigidBody {
    RigidBody {
        frame,
        velocity,
        mass,
        shape: ColliderShape::Cuboid { half_extents },
        ..Default::default()
    }
}

pub(super) fn assert_vec3_approx(a: Vec3, b: Vec3) {
    assert!((a - b).length() < 1e-4, "expected {b:?}, got {a:?}");
}

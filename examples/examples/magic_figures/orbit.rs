//! Orbit tags and the circular path shapes follow.

use crate::ShapeSpec;
use meridian_sdk::Vec3;
use meridian_sdk::dsl::dsl_tag;

pub(crate) const ORBIT_RADIUS: f32 = 3.2;

const ORBIT_HEIGHT: f32 = 2.0;

/// Full orbit period, seconds — slow enough to watch each shape's own
/// spin independently of the group's motion.
const ORBIT_PERIOD: f32 = 24.0;

/// This example's own orbital-motion tag — `phase`/`spin_speed` are
/// meaningless outside this specific "shapes orbiting a point" demo,
/// exactly the kind of thing that belongs in the example, not the SDK.
#[dsl_tag(name = "Orbit")]
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct OrbitTag {
    pub(crate) phase: f32,
    pub(crate) spin_speed: f32,
}

/// This frame's world position for `shape` — a circular orbit around
/// the origin at `ORBIT_HEIGHT`, phase-separated per [`ShapeSpec::phase`](crate::ShapeSpec::phase).
pub(crate) fn orbit_position(shape: &ShapeSpec, elapsed: f32) -> Vec3 {
    let angle = elapsed / ORBIT_PERIOD * std::f32::consts::TAU + shape.phase;
    Vec3::new(
        ORBIT_RADIUS * angle.cos(),
        ORBIT_HEIGHT,
        ORBIT_RADIUS * angle.sin(),
    )
}

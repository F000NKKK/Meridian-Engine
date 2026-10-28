//! A pair of bodies that began touching.

use meridian_gac::Vec3;

/// Two bodies began touching during a step. A physics fact only: it says
/// nothing about sound, particles or damage; consumers decide what it means.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ContactEvent {
    /// Index of the first body in [`PhysicsSubsystem::bodies`](crate::PhysicsSubsystem::bodies); always the smaller index.
    pub a: usize,
    /// Index of the second body; always the larger index.
    pub b: usize,
    /// The deepest contact point, in world space.
    pub point: Vec3,
    /// Contact normal pointing from `a` toward `b`.
    pub normal: Vec3,
    /// How fast the bodies were closing along `normal` before the step, in
    /// world units per second; never negative.
    pub impact_speed: f32,
}

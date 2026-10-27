//! Shared test bodies and subsystems.

use meridian_audio_core::{Mixer, SpeakerLayout};
use meridian_gac_core::{Motor3, Vec3};
use meridian_physics_core::{ColliderShape, RigidBody};

use crate::*;

/// A compute context on the deterministic single-threaded fake device.
pub(crate) fn sequential_context() -> meridian_compute_runtime::ComputeContext {
    meridian_compute_runtime::ComputeContext::new(std::sync::Arc::new(
        meridian_compute_runtime::testing::SequentialDevice,
    ))
}

pub(crate) fn falling_body() -> RigidBody {
    RigidBody {
        frame: Motor3::translation(Vec3::new(0.0, 10.0, 0.0)),
        mass: 1.0,
        shape: ColliderShape::Sphere { radius: 0.5 },
        ..Default::default()
    }
}

pub(crate) fn subsystem_pair() -> (PhysicsSubsystem, AudioSubsystem) {
    (
        PhysicsSubsystem::default(),
        AudioSubsystem::new(Mixer::new(SpeakerLayout::mono())),
    )
}

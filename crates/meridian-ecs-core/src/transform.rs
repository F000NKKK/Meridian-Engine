//! The spatial component: a motor.

use meridian_gac_core::Motor3;

use crate::Component;

/// The engine-wide spatial component: rotation + translation as a single
/// motor, shared with physics/graphics/audio (see docs/gac-design.md).
#[derive(Debug, Clone, Copy, Default)]
pub struct Transform {
    pub motor: Motor3,
}

impl Component for Transform {}

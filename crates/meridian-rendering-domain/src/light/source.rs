//! A light as an ECS component.

use meridian_ecs::Component;

use super::LightKind;

/// Makes an entity a light: where it sits and where it points come from the
/// entity's own transform, [`extract_lights`] turns it into a [`Light`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LightSource {
    pub kind: LightKind,
    /// Linear RGB.
    pub color: [f32; 3],
    /// Scales `color`.
    pub intensity: f32,
}

impl Component for LightSource {}

//! Gain falling with distance from the listener.

/// Distance attenuation. OpenAL's "inverse clamped distance" model:
/// `gain == 1.0` at or inside `reference_distance`, decreasing beyond it,
/// clamped at `max_distance` so a source doesn't silently vanish to
/// exactly zero.
#[derive(Debug, Clone, Copy)]
pub struct AttenuationModel {
    pub reference_distance: f32,
    pub rolloff: f32,
    pub max_distance: f32,
}

impl Default for AttenuationModel {
    fn default() -> Self {
        Self {
            reference_distance: 1.0,
            rolloff: 1.0,
            max_distance: 1000.0,
        }
    }
}

impl AttenuationModel {
    pub fn gain(&self, distance: f32) -> f32 {
        let d = distance.clamp(self.reference_distance, self.max_distance);
        self.reference_distance
            / (self.reference_distance + self.rolloff * (d - self.reference_distance))
    }
}

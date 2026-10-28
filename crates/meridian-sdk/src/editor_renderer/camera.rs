//! The orbit camera of an editor viewport.

use meridian_gac::{Motor3, Projection, Vec3};
use meridian_rendering_domain::{Camera, look_at_upright};

/// The farthest the camera tilts from the horizon, so it never flips over.
const MAX_PITCH: f32 = 1.5;

/// A camera orbiting `target`: `yaw` turns around +Y (0 looks down -Z from +Z),
/// `pitch` lifts it above the horizon (radians, +Y up).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct EditorCamera {
    pub target: [f32; 3],
    pub yaw: f32,
    pub pitch: f32,
    pub distance: f32,
    /// Vertical field of view, radians.
    pub fov_y: f32,
}

impl Default for EditorCamera {
    fn default() -> Self {
        Self {
            target: [0.0; 3],
            yaw: 0.6,
            pitch: 0.35,
            distance: 8.0,
            fov_y: 1.0,
        }
    }
}

impl EditorCamera {
    /// The default angle, backed off so a sphere of `radius` around `center`
    /// fills the view.
    #[must_use]
    pub fn framing(center: [f32; 3], radius: f32) -> Self {
        let base = Self::default();
        let distance = radius.max(0.001) / (base.fov_y * 0.5).sin() * 0.65;
        Self {
            target: center,
            distance,
            ..base
        }
    }

    /// Where the eye sits.
    #[must_use]
    pub fn eye(&self) -> Vec3 {
        let pitch = self.pitch.clamp(-MAX_PITCH, MAX_PITCH);
        let target = Vec3::new(self.target[0], self.target[1], self.target[2]);
        target
            + Vec3::new(
                self.distance * pitch.cos() * self.yaw.sin(),
                self.distance * pitch.sin(),
                self.distance * pitch.cos() * self.yaw.cos(),
            )
    }

    /// The camera for a frame of `aspect` (width over height).
    #[must_use]
    pub fn camera(&self, aspect: f32) -> Camera {
        let eye = self.eye();
        let target = Vec3::new(self.target[0], self.target[1], self.target[2]);
        Camera {
            frame: Motor3::from_rotation_translation(look_at_upright(eye, target), eye),
            projection: Projection::perspective(
                self.fov_y,
                aspect,
                (self.distance * 0.01).max(0.01),
                self.distance * 200.0 + 100.0,
            ),
        }
    }
}

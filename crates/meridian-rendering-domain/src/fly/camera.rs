//! A free-fly camera.
//!
//! The camera uses `gac-core`'s local-forward `+X` convention (see
//! [`look_at_rotor`](crate::look_at_rotor) and [`FlyCamera::yaw`]'s own doc) and composes its
//! view rotor as yaw-then-pitch around the yawed local-right axis,
//! avoiding the roll problem [`look_at_rotor`](crate::look_at_rotor) has when pitch changes.

use meridian_gac::{Motor3, Vec3};

use super::FlyControls;
use crate::Camera;

/// A free-fly camera: WASD to move (relative to the way it's currently
/// looking, not world axes), Space/Ctrl for up/down, mouse to look
/// around — the caller is expected to grab the cursor
/// (`Window::set_cursor_grabbed(true)` once in `on_ready`) so the mouse
/// steers the view directly instead of a visible cursor wandering off
/// the window. Exists because a fixed `look_at_rotor(eye, target)`
/// camera gives no way to inspect a scene from angles the author didn't
/// hardcode — both `magic_figures` and `physic_figures` want to walk
/// around and look at their scenes from any angle.
#[derive(Debug)]
pub struct FlyCamera {
    pub position: Vec3,
    /// Radians, measured from `+X` toward `+Z` (matches `Vec3::X` being
    /// `gac-core`'s local-forward convention — see [`look_at_rotor`](crate::look_at_rotor)).
    pub yaw: f32,
    /// Radians, clamped just short of &plusmn;90&deg; to avoid a gimbal
    /// flip at the poles.
    pub pitch: f32,
    pub move_speed: f32,
    pub look_sensitivity: f32,
}

impl FlyCamera {
    #[must_use]
    pub fn new(position: Vec3) -> Self {
        // Facing roughly toward -Z (a scene's contents in these examples
        // sit near the origin along -Z from a positive-Z starting eye
        // point) with a slight downward tilt, a reasonable default that
        // doesn't start looking at empty sky.
        Self {
            position,
            yaw: -core::f32::consts::FRAC_PI_2,
            pitch: -0.2,
            move_speed: 3.0,
            look_sensitivity: 0.003,
        }
    }

    /// Advances the camera by one frame of `controls`/`dt`. Call once per
    /// redraw, after computing `dt` and before building this frame's
    /// [`Camera`] via [`Self::camera`]. `controls.look` is device-relative
    /// mouse motion, not cursor-position-based, so it keeps working once the
    /// caller grabs the cursor — a locked cursor stops generating position
    /// deltas entirely.
    pub fn update(&mut self, controls: FlyControls, dt: f32) {
        let (dx, dy) = controls.look;
        self.yaw += dx * self.look_sensitivity;
        self.pitch = (self.pitch - dy * self.look_sensitivity).clamp(
            -core::f32::consts::FRAC_PI_2 + 0.01,
            core::f32::consts::FRAC_PI_2 - 0.01,
        );

        let cy = self.yaw.cos();
        let sy = self.yaw.sin();
        let forward_horiz = Vec3::new(cy, 0.0, sy);
        let right = Vec3::new(-sy, 0.0, cy);
        let mut movement = Vec3::ZERO;
        if controls.forward {
            movement = movement + forward_horiz;
        }
        if controls.backward {
            movement = movement - forward_horiz;
        }
        if controls.right {
            movement = movement + right;
        }
        if controls.left {
            movement = movement - right;
        }
        if controls.up {
            movement = movement + Vec3::Y;
        }
        if controls.down {
            movement = movement - Vec3::Y;
        }
        if movement.length() > 1e-5 {
            let speed = if controls.boost {
                self.move_speed * 3.0
            } else {
                self.move_speed
            };
            self.position = self.position + movement.normalize() * speed * dt;
        }
    }

    /// The view rotor: yaw around world `+Y`, *then* pitch around the
    /// yaw-rotated local-right axis — not [`look_at_rotor`](crate::look_at_rotor). That
    /// helper picks the shortest-arc rotation from local-forward to a
    /// target direction, which is ambiguous about roll (many rotations
    /// take `+X` to a given `forward`, and the shortest-arc choice drifts
    /// between them as pitch changes) — it visibly rolled the view
    /// clockwise/counter-clockwise while pitching up/down. Composing an
    /// explicit yaw-then-pitch instead is standard FPS-camera
    /// construction: rotating around a horizontal axis that's itself
    /// already been yawed never introduces roll, by construction (the
    /// world-up direction stays consistently "up" in the resulting view
    /// at every pitch).
    ///
    /// Note: `Rotor::from_axis_angle(Vec3::Y, yaw)` produces a
    /// **left-handed** rotation (a known asymmetry in `gac-core`'s
    /// `from_axis_angle` for the Y axis), so the yaw is negated here to
    /// keep the rotor consistent with the ad‑hoc `forward_horiz` basis
    /// used in [`Self::update`] (which follows a conventional right-handed
    /// +X‑toward‑+Z yaw).
    pub(super) fn rotor(&self) -> meridian_gac::Rotor {
        use meridian_gac::Rotor;
        let yaw_rotor = Rotor::from_axis_angle(Vec3::Y, -self.yaw);
        let right_after_yaw = yaw_rotor.transform_vector(Vec3::Z);
        let pitch_rotor = Rotor::from_axis_angle(right_after_yaw, self.pitch);
        yaw_rotor.compose(pitch_rotor)
    }

    #[must_use]
    pub fn camera(&self, aspect_ratio: f32) -> Camera {
        Camera {
            frame: Motor3::from_rotation_translation(self.rotor(), self.position),
            projection: meridian_gac::Projection::perspective(
                55.0_f32.to_radians(),
                aspect_ratio,
                0.1,
                100.0,
            ),
        }
    }
}

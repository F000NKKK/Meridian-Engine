//! A free-fly camera and its `look_at_rotor` helper — extracted from
//! `lib.rs` so the camera logic and tests are their own file, keeping
//! `lib.rs` focused on the soft-body rendering scaffolding that several
//! examples share.
//!
//! The camera uses `gac-core`'s local-forward `+X` convention (see
//! `look_at_rotor` and [`FlyCamera::yaw`]'s own doc) and composes its
//! view rotor as yaw-then-pitch around the yawed local-right axis,
//! avoiding the roll problem [`look_at_rotor`] has when pitch changes.

use meridian_gac_core::{Motor3, Vec3};
use meridian_graphics_core::Camera;
use meridian_platform_core::{InputState, KeyCode};

/// A camera rotor turning `gac-core`'s local-forward `+X` toward `target
/// - eye` — see `spinning_cube`'s identical helper, duplicated here
/// (not worth a shared engine-level utility for three examples) rather
/// than depended on across two ``[[example]]`` binaries.
pub fn look_at_rotor(eye: Vec3, target: Vec3) -> meridian_gac_core::Rotor {
    use meridian_gac_core::Rotor;
    let forward = (target - eye).normalize();
    let local_forward = Vec3::X;
    let cos_angle = local_forward.dot(forward).clamp(-1.0, 1.0);
    if cos_angle > 0.9999 {
        return Rotor::identity();
    }
    if cos_angle < -0.9999 {
        return Rotor::from_axis_angle(Vec3::Y, core::f32::consts::PI);
    }
    let axis = local_forward.cross(forward).normalize();
    Rotor::from_axis_angle(axis, cos_angle.acos())
}

/// A free-fly camera: WASD to move (relative to the way it's currently
/// looking, not world axes), Space/Ctrl for up/down, mouse to look
/// around — the caller is expected to grab the cursor
/// (`Window::set_cursor_grabbed(true)` once in `on_ready`) so the mouse
/// steers the view directly instead of a visible cursor wandering off
/// the window. Exists because a fixed `look_at_rotor(eye, target)`
/// camera (`spinning_cube`'s approach) gives no way to inspect a scene
/// from angles the author didn't hardcode — the soft-body examples need
/// that inspection more than a spinning cube does, since "is this ball
/// actually deforming/jiggling or just sitting there" is exactly the
/// kind of thing you want to walk around and look at.
pub struct FlyCamera {
    pub position: Vec3,
    /// Radians, measured from `+X` toward `+Z` (matches `Vec3::X` being
    /// `gac-core`'s local-forward convention — see [`look_at_rotor`]).
    pub yaw: f32,
    /// Radians, clamped just short of &plusmn;90&deg; to avoid a gimbal
    /// flip at the poles.
    pub pitch: f32,
    pub move_speed: f32,
    pub look_sensitivity: f32,
}

impl FlyCamera {
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

    /// Advances the camera by one frame of `input`/`dt`. Call once per
    /// `on_redraw`, after computing `dt` and before building this
    /// frame's [`Camera`] via [`Self::camera`]. Reads
    /// [`InputState::raw_mouse_delta`] (device-relative motion, not
    /// cursor-position-based) so this keeps working once the caller
    /// grabs the cursor via `Window::set_cursor_grabbed(true)` in
    /// `on_ready` (the expected setup — see this type's own doc comment)
    /// — a locked cursor stops generating position deltas entirely.
    pub fn update(&mut self, input: &InputState, dt: f32) {
        let (dx, dy) = input.raw_mouse_delta();
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
        if input.is_key_down(KeyCode::W) {
            movement = movement + forward_horiz;
        }
        if input.is_key_down(KeyCode::S) {
            movement = movement - forward_horiz;
        }
        if input.is_key_down(KeyCode::D) {
            movement = movement + right;
        }
        if input.is_key_down(KeyCode::A) {
            movement = movement - right;
        }
        if input.is_key_down(KeyCode::Space) {
            movement = movement + Vec3::Y;
        }
        if input.is_key_down(KeyCode::ControlLeft) {
            movement = movement - Vec3::Y;
        }
        if movement.length() > 1e-5 {
            let speed = if input.is_key_down(KeyCode::ShiftLeft) {
                self.move_speed * 3.0
            } else {
                self.move_speed
            };
            self.position = self.position + movement.normalize() * speed * dt;
        }
    }

    /// The view rotor: yaw around world `+Y`, *then* pitch around the
    /// yaw-rotated local-right axis — not [`look_at_rotor`]. That
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
    fn rotor(&self) -> meridian_gac_core::Rotor {
        use meridian_gac_core::Rotor;
        let yaw_rotor = Rotor::from_axis_angle(Vec3::Y, -self.yaw);
        let right_after_yaw = yaw_rotor.transform_vector(Vec3::Z);
        let pitch_rotor = Rotor::from_axis_angle(right_after_yaw, self.pitch);
        yaw_rotor.compose(pitch_rotor)
    }

    pub fn camera(&self, aspect_ratio: f32) -> Camera {
        Camera {
            frame: Motor3::from_rotation_translation(self.rotor(), self.position),
            projection: meridian_gac_core::Projection::perspective(
                55.0_f32.to_radians(),
                aspect_ratio,
                0.1,
                100.0,
            ),
        }
    }
}

#[cfg(test)]
mod fly_camera_tests {
    use super::*;
    use meridian_platform_core::{InputState, KeyCode};

    /// Returns (forward_horiz, right, up) expected from the camera's yaw.
    fn expected_basis(yaw: f32) -> (Vec3, Vec3, Vec3) {
        let cy = yaw.cos();
        let sy = yaw.sin();
        (Vec3::new(cy, 0.0, sy), Vec3::new(-sy, 0.0, cy), Vec3::Y)
    }

    #[test]
    fn rotor_and_forward_horiz_agree() {
        // Verify that the ad-hoc forward_horiz computation matches
        // what rotor() actually produces for the horizontal component.
        let yaws = [
            0.0,
            core::f32::consts::FRAC_PI_4,
            core::f32::consts::FRAC_PI_2,
            3.0 * core::f32::consts::FRAC_PI_4,
            core::f32::consts::PI,
            -core::f32::consts::FRAC_PI_4,
            -core::f32::consts::FRAC_PI_2,
            -3.0 * core::f32::consts::FRAC_PI_4,
        ];
        for &yaw in &yaws {
            let cam = FlyCamera {
                position: Vec3::ZERO,
                yaw,
                pitch: 0.0,
                move_speed: 3.0,
                look_sensitivity: 0.003,
            };
            let rotor_forward = cam.rotor().transform_vector(Vec3::X);
            let cy = yaw.cos();
            let sy = yaw.sin();
            let expected_forward_horiz = Vec3::new(cy, 0.0, sy);
            // Compare XZ components (Y may differ if pitch were non-zero, but pitch=0 here)
            assert!(
                (rotor_forward.x - expected_forward_horiz.x).abs() < 1e-6,
                "X mismatch at yaw={}: rotor_forward.x={} expected={}",
                yaw,
                rotor_forward.x,
                expected_forward_horiz.x
            );
            assert!(
                (rotor_forward.z - expected_forward_horiz.z).abs() < 1e-6,
                "Z mismatch at yaw={}: rotor_forward.z={} expected={}",
                yaw,
                rotor_forward.z,
                expected_forward_horiz.z
            );
            // Y should be 0 at pitch=0
            assert!(
                rotor_forward.y.abs() < 1e-6,
                "Y should be 0 at pitch=0, yaw={}, got {}",
                yaw,
                rotor_forward.y
            );
        }
    }

    fn simulate(dt: f32, speed: f32, keys: &[KeyCode]) -> Vec3 {
        let mut cam = FlyCamera::new(Vec3::ZERO);
        cam.move_speed = speed;
        let mut input = InputState::new();
        for &k in keys {
            input.press_key(k);
        }
        cam.update(&input, dt);
        cam.position
    }

    fn simulate_at(yaw: f32, dt: f32, speed: f32, keys: &[KeyCode]) -> Vec3 {
        let mut cam = FlyCamera::new(Vec3::ZERO);
        cam.yaw = yaw;
        cam.move_speed = speed;
        let mut input = InputState::new();
        for &k in keys {
            input.press_key(k);
        }
        cam.update(&input, dt);
        cam.position
    }

    #[test]
    fn no_keys_does_not_move() {
        let pos = simulate(1.0, 3.0, &[]);
        assert_eq!(pos, Vec3::ZERO);
    }

    #[test]
    fn w_moves_forward_along_yaw_basis() {
        // Test at 8 different yaw angles covering all quadrants
        let yaws = [
            0.0,
            core::f32::consts::FRAC_PI_4,
            core::f32::consts::FRAC_PI_2,
            3.0 * core::f32::consts::FRAC_PI_4,
            core::f32::consts::PI,
            -core::f32::consts::FRAC_PI_4,
            -core::f32::consts::FRAC_PI_2,
            -3.0 * core::f32::consts::FRAC_PI_4,
        ];
        for &yaw in &yaws {
            let pos = simulate_at(yaw, 1.0, 3.0, &[KeyCode::W]);
            let (fwd, _, _) = expected_basis(yaw);
            let expected = fwd.normalize() * 3.0;
            assert!(
                (pos - expected).length() < 1e-5,
                "W yaw={} got {:?} expected {:?}",
                yaw,
                pos,
                expected
            );
        }
    }

    #[test]
    fn s_moves_backward_opposite_to_w() {
        let yaws = [
            0.0,
            core::f32::consts::FRAC_PI_2,
            core::f32::consts::PI,
            -core::f32::consts::FRAC_PI_2,
        ];
        for &yaw in &yaws {
            let fwd = simulate_at(yaw, 1.0, 3.0, &[KeyCode::W]);
            let bwd = simulate_at(yaw, 1.0, 3.0, &[KeyCode::S]);
            assert!(
                (fwd + bwd).length() < 1e-5,
                "S should oppose W at yaw={}",
                yaw
            );
        }
    }

    #[test]
    fn d_moves_right_along_yaw_basis() {
        let yaws = [
            0.0,
            core::f32::consts::FRAC_PI_4,
            core::f32::consts::FRAC_PI_2,
            core::f32::consts::PI,
            -core::f32::consts::FRAC_PI_2,
        ];
        for &yaw in &yaws {
            let pos = simulate_at(yaw, 1.0, 3.0, &[KeyCode::D]);
            let (_, right, _) = expected_basis(yaw);
            let expected = right.normalize() * 3.0;
            assert!(
                (pos - expected).length() < 1e-5,
                "D yaw={} got {:?} expected {:?}",
                yaw,
                pos,
                expected
            );
        }
    }

    #[test]
    fn a_moves_left_opposite_to_d() {
        let yaws = [
            0.0,
            core::f32::consts::FRAC_PI_2,
            core::f32::consts::PI,
            -core::f32::consts::FRAC_PI_2,
        ];
        for &yaw in &yaws {
            let d = simulate_at(yaw, 1.0, 3.0, &[KeyCode::D]);
            let a = simulate_at(yaw, 1.0, 3.0, &[KeyCode::A]);
            assert!((d + a).length() < 1e-5, "A should oppose D at yaw={}", yaw);
        }
    }

    #[test]
    fn space_moves_up_and_ctrl_moves_down() {
        let up = simulate(1.0, 3.0, &[KeyCode::Space]);
        assert_eq!(up, Vec3::new(0.0, 3.0, 0.0));

        let down = simulate(1.0, 3.0, &[KeyCode::ControlLeft]);
        assert_eq!(down, Vec3::new(0.0, -3.0, 0.0));

        let both = simulate(1.0, 3.0, &[KeyCode::Space, KeyCode::ControlLeft]);
        assert_eq!(both, Vec3::ZERO);
    }

    #[test]
    fn diagonal_movement_preserves_length() {
        let pos = simulate_at(0.0, 1.0, 3.0, &[KeyCode::W, KeyCode::D]);
        let expected = (Vec3::X + Vec3::Z).normalize() * 3.0;
        assert!(
            (pos - expected).length() < 1e-5,
            "W+D diagonal got {:?}",
            pos
        );
    }

    #[test]
    fn full_3d_movement_wsad_space_ctrl() {
        let yaws = [
            0.0,
            core::f32::consts::FRAC_PI_2,
            core::f32::consts::PI,
            -core::f32::consts::FRAC_PI_2,
        ];
        for &yaw in &yaws {
            let (fwd, right, _) = expected_basis(yaw);
            let pos = simulate_at(yaw, 1.0, 3.0, &[KeyCode::W, KeyCode::D, KeyCode::Space]);
            let expected = (fwd.normalize() + right.normalize() + Vec3::Y).normalize() * 3.0;
            assert!(
                (pos - expected).length() < 1e-5,
                "W+D+Space yaw={} got {:?} expected {:?}",
                yaw,
                pos,
                expected
            );
        }
    }

    #[test]
    fn pitch_does_not_affect_movement_direction() {
        let pitches = [0.0, 0.5, -0.3, 1.2, -1.0];
        for &pitch in &pitches {
            let pos = simulate_at(0.0, 1.0, 3.0, &[KeyCode::W]);
            let expected = Vec3::new(3.0, 0.0, 0.0);
            assert!(
                (pos - expected).length() < 1e-5,
                "W at pitch={} got {:?}",
                pitch,
                pos
            );
        }
    }

    #[test]
    fn move_speed_scales_velocity() {
        let slow = simulate(1.0, 1.0, &[KeyCode::W]);
        let fast = simulate(1.0, 10.0, &[KeyCode::W]);
        assert!((fast - slow * 10.0).length() < 1e-5);
    }

    #[test]
    fn dt_scales_velocity() {
        let t1 = simulate(0.5, 3.0, &[KeyCode::W]);
        let t2 = simulate(1.0, 3.0, &[KeyCode::W]);
        assert!((t2 - t1 * 2.0).length() < 1e-5);
    }

    #[test]
    fn shift_multiplies_move_speed() {
        let normal = simulate(1.0, 3.0, &[KeyCode::W]);
        let shifted = simulate(1.0, 3.0, &[KeyCode::W, KeyCode::ShiftLeft]);
        assert!((shifted - normal * 3.0).length() < 1e-5);
    }

    #[test]
    fn mouse_affects_yaw() {
        let mut cam = FlyCamera::new(Vec3::ZERO);
        let start_yaw = cam.yaw;
        let mut input = InputState::new();
        input.accumulate_mouse_motion(100.0, 0.0);
        cam.update(&input, 1.0);
        assert!(
            (cam.yaw - start_yaw).abs() > 0.001,
            "mouse dx should change yaw"
        );
    }

    #[test]
    fn mouse_affects_pitch() {
        let mut cam = FlyCamera::new(Vec3::ZERO);
        let start_pitch = cam.pitch;
        let mut input = InputState::new();
        input.accumulate_mouse_motion(0.0, 100.0);
        cam.update(&input, 1.0);
        assert!(
            (cam.pitch - start_pitch).abs() > 0.001,
            "mouse dy should change pitch"
        );
    }

    #[test]
    fn pitch_clamping() {
        let mut cam = FlyCamera::new(Vec3::ZERO);
        let mut input = InputState::new();
        // Try to exceed ±90°
        input.accumulate_mouse_motion(0.0, -1e6);
        cam.update(&input, 1.0);
        assert!(cam.pitch > -core::f32::consts::FRAC_PI_2 + 0.009);
        input.accumulate_mouse_motion(0.0, 1e6);
        cam.update(&input, 1.0);
        assert!(cam.pitch < core::f32::consts::FRAC_PI_2 - 0.009);
    }

    #[test]
    fn zero_movement_does_not_change_position() {
        let mut cam = FlyCamera::new(Vec3::new(10.0, -5.0, 3.0));
        let input = InputState::new();
        cam.update(&input, 1.0);
        assert_eq!(cam.position, Vec3::new(10.0, -5.0, 3.0));
    }

    #[test]
    fn w_s_d_a_with_nonzero_pitch_stays_horizontal() {
        let yaws = [0.0, core::f32::consts::FRAC_PI_2, core::f32::consts::PI];
        for &yaw in &yaws {
            for &pitch in &[0.3, -0.5] {
                let pos = simulate_at(
                    yaw,
                    1.0,
                    3.0,
                    &[KeyCode::W, KeyCode::S, KeyCode::D, KeyCode::A],
                );
                // All four buttons cancel out → no movement regardless of pitch
                assert!(
                    pos.length() < 1e-5,
                    "W+S+D+A yaw={} pitch={} should stay at origin, got {:?}",
                    yaw,
                    pitch,
                    pos
                );
            }
        }
    }
}

//! Tests of this module.

use meridian_platform_core::{InputState, KeyCode};

use super::*;

/// Returns (`forward_horiz`, right, up) expected from the camera's yaw.
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
            "W yaw={yaw} got {pos:?} expected {expected:?}"
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
            "S should oppose W at yaw={yaw}"
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
            "D yaw={yaw} got {pos:?} expected {expected:?}"
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
        assert!((d + a).length() < 1e-5, "A should oppose D at yaw={yaw}");
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
    assert!((pos - expected).length() < 1e-5, "W+D diagonal got {pos:?}");
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
            "W+D+Space yaw={yaw} got {pos:?} expected {expected:?}"
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
            "W at pitch={pitch} got {pos:?}"
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
                "W+S+D+A yaw={yaw} pitch={pitch} should stay at origin, got {pos:?}"
            );
        }
    }
}

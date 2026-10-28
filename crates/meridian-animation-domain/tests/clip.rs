//! Tests of keyframes, clips and playback.

use core::f32::consts::FRAC_PI_2;

use meridian_animation_domain::{
    Channel, Clip, ClipError, Interpolation, JointId, JointTransform, Keys, Playback, Pose, Repeat,
    Skeleton, Track,
};
use meridian_gac::{Rotor, Vec3};

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-5
}

fn keys(times: &[f32], values: &[[f32; 3]], interpolation: Interpolation) -> Track {
    Track::Translation(
        Keys::new(times.to_vec(), values.to_vec(), interpolation).expect("a valid test fixture"),
    )
}

fn sampled(track: &Track, time: f32) -> Vec3 {
    let mut transform = JointTransform::default();
    track.apply(time, &mut transform);
    transform.translation
}

#[test]
fn keys_need_increasing_times_and_matching_values() {
    let ok = |t: &[f32], v: usize, i| Keys::new(t.to_vec(), vec![[0.0f32; 3]; v], i);
    assert_eq!(
        ok(&[], 0, Interpolation::Linear).unwrap_err(),
        ClipError::NoKeys
    );
    assert_eq!(
        ok(&[0.0, 1.0], 3, Interpolation::Linear).unwrap_err(),
        ClipError::ValueCount { keys: 2, values: 3 }
    );
    assert_eq!(
        ok(&[0.0, 1.0], 2, Interpolation::CubicSpline).unwrap_err(),
        ClipError::ValueCount { keys: 2, values: 2 }
    );
    assert_eq!(
        ok(&[0.0, 1.0, 1.0], 3, Interpolation::Linear).unwrap_err(),
        ClipError::TimesNotIncreasing { index: 2 }
    );
    assert_eq!(
        ok(&[0.0, f32::NAN], 2, Interpolation::Linear).unwrap_err(),
        ClipError::TimesNotIncreasing { index: 1 }
    );
    assert!(ok(&[0.0, 1.0], 6, Interpolation::CubicSpline).is_ok());
}

#[test]
fn linear_keys_blend_and_clamp_at_both_ends() {
    let track = keys(
        &[1.0, 3.0],
        &[[0.0, 0.0, 0.0], [4.0, 0.0, 8.0]],
        Interpolation::Linear,
    );
    assert!(close(sampled(&track, 2.0).x, 2.0));
    assert!(close(sampled(&track, 2.0).z, 4.0));
    assert!(close(sampled(&track, 0.0).x, 0.0), "before the first key");
    assert!(close(sampled(&track, 9.0).x, 4.0), "after the last key");
}

#[test]
fn step_keys_hold_their_value_until_the_next_key() {
    let track = keys(
        &[0.0, 1.0, 2.0],
        &[[1.0; 3], [2.0; 3], [3.0; 3]],
        Interpolation::Step,
    );
    assert!(close(sampled(&track, 0.99).x, 1.0));
    assert!(close(sampled(&track, 1.0).x, 2.0));
    assert!(close(sampled(&track, 1.5).x, 2.0));
}

#[test]
fn a_spline_with_zero_tangents_is_halfway_at_the_middle_and_flat_at_the_keys() {
    let zero = [0.0; 3];
    let track = keys(
        &[0.0, 1.0],
        &[zero, [0.0; 3], zero, zero, [1.0, 0.0, 0.0], zero],
        Interpolation::CubicSpline,
    );
    assert!(close(sampled(&track, 0.5).x, 0.5));
    assert!(close(sampled(&track, 0.0).x, 0.0));
    assert!(close(sampled(&track, 1.0).x, 1.0));
    assert!(sampled(&track, 0.1).x < 0.1, "eases in");
}

#[test]
fn a_spline_follows_its_tangents() {
    // p0 = 0 leaving at slope 1, p1 = 0 arriving at slope -1, over 2 seconds:
    // the Hermite value at the middle is 0.25 + 0.25 = 0.5.
    let slope = |x: f32| [x, 0.0, 0.0];
    let track = keys(
        &[0.0, 2.0],
        &[
            slope(0.0),
            slope(0.0),
            slope(1.0),
            slope(-1.0),
            slope(0.0),
            slope(0.0),
        ],
        Interpolation::CubicSpline,
    );
    assert!(close(sampled(&track, 1.0).x, 0.5));
}

#[test]
fn rotation_keys_take_the_short_arc_at_constant_speed() {
    let half = |angle: f32| [0.0, 0.0, (angle / 2.0).sin(), (angle / 2.0).cos()];
    let track = Track::Rotation(
        Keys::new(
            vec![0.0, 1.0],
            vec![half(0.0), half(FRAC_PI_2)],
            Interpolation::Linear,
        )
        .expect("a valid test fixture"),
    );
    let mut transform = JointTransform::default();
    track.apply(0.5, &mut transform);
    let want = Rotor::from_axis_angle(Vec3::Z, FRAC_PI_2 / 2.0).transform_vector(Vec3::X);
    let got = transform.rotation.transform_vector(Vec3::X);
    assert!(close(got.x, want.x) && close(got.y, want.y) && close(got.z, want.z));
}

#[test]
fn a_clip_is_as_long_as_its_latest_key_and_leaves_other_properties_alone() {
    let channel = |index: usize, track| Channel {
        target: JointId::from_index(index).expect("a valid test fixture"),
        track,
    };
    let clip = Clip::new(vec![
        channel(
            0,
            keys(
                &[0.0, 2.0],
                &[[0.0; 3], [2.0, 0.0, 0.0]],
                Interpolation::Linear,
            ),
        ),
        channel(
            7,
            keys(&[0.0, 5.0], &[[0.0; 3], [1.0; 3]], Interpolation::Linear),
        ),
    ]);
    assert!(close(clip.duration(), 5.0));

    let skeleton = Skeleton::default();
    let mut pose = Pose::rest(&skeleton);
    clip.sample(1.0, &mut pose); // an empty pose: every channel is skipped, nothing panics
    assert!(pose.transforms().is_empty());
}

#[test]
fn sampling_a_clip_moves_only_the_animated_joint() {
    let skeleton = Skeleton::new(
        vec![meridian_animation_domain::Joint {
            parent: None,
            rest: JointTransform {
                scale: Vec3::new(2.0, 2.0, 2.0),
                ..JointTransform::default()
            },
            inverse_bind: [
                [1.0, 0.0, 0.0, 0.0],
                [0.0, 1.0, 0.0, 0.0],
                [0.0, 0.0, 1.0, 0.0],
                [0.0, 0.0, 0.0, 1.0],
            ],
        }],
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ],
    )
    .expect("a valid test fixture");
    let clip = Clip::new(vec![Channel {
        target: JointId::from_index(0).expect("a valid test fixture"),
        track: keys(
            &[0.0, 1.0],
            &[[0.0; 3], [3.0, 0.0, 0.0]],
            Interpolation::Linear,
        ),
    }]);
    let mut pose = Pose::rest(&skeleton);
    clip.sample(0.5, &mut pose);
    let joint = pose.transforms()[0];
    assert!(close(joint.translation.x, 1.5));
    assert!(close(joint.scale.x, 2.0), "scale was not animated");
}

#[test]
fn playback_loops_clamps_and_runs_backwards() {
    let mut looping = Playback::default();
    looping.advance(2.5, 2.0);
    assert!(close(looping.time, 0.5));
    assert!(!looping.finished(2.0));

    let mut once = Playback {
        repeat: Repeat::Once,
        ..Playback::default()
    };
    once.advance(5.0, 2.0);
    assert!(close(once.time, 2.0));
    assert!(once.finished(2.0));

    let mut backwards = Playback {
        speed: -1.0,
        time: 0.5,
        ..Playback::default()
    };
    backwards.advance(1.0, 2.0);
    assert!(close(backwards.time, 1.5), "wraps below zero");

    let mut empty = Playback::default();
    empty.advance(1.0, 0.0);
    assert!(close(empty.time, 0.0));
}

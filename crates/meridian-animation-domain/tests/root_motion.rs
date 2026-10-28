//! Tests of root motion: the travel an animation carries a joint along.

use meridian_animation_domain::{
    Animation, AnimationLayer, AnimationPlayer, Channel, Clip, Interpolation, Joint, JointId,
    JointMatrix, JointTransform, Keys, Repeat, Rig, RootMotion, Skeleton, SkeletonClip, Track,
};

const IDENTITY: JointMatrix = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// One joint that walks from x = 0 to x = 4 over 4 seconds.
fn walker() -> (Rig, RootMotion) {
    let keys = Keys::<3>::new(
        vec![0.0, 4.0],
        vec![[0.0; 3], [4.0, 0.0, 0.0]],
        Interpolation::Linear,
    )
    .expect("valid keys");
    let joint_id = JointId::from_index(0).expect("joint 0");
    let joint = Joint {
        parent: None,
        rest: JointTransform::default(),
        inverse_bind: IDENTITY,
    };
    let rig = Rig {
        skeletons: vec![Skeleton::new(vec![joint], IDENTITY).expect("a valid skeleton")],
        animations: vec![Animation {
            clips: vec![SkeletonClip {
                skeleton: 0,
                clip: Clip::new(vec![Channel {
                    target: joint_id,
                    track: Track::Translation(keys),
                }]),
            }],
            weights: Vec::new(),
        }],
        rest_weights: Vec::new(),
    };
    let pin = RootMotion {
        skeleton: 0,
        joint: joint_id,
        follow_yaw: false,
    };
    (rig, pin)
}

#[test]
fn the_player_reports_how_far_the_joint_walked() {
    let (rig, pin) = walker();
    let mut player = AnimationPlayer::new(Some(0));
    let travel = player.advance_travel(&rig, 1.0, pin);
    assert!(
        (travel.translation.x - 1.0).abs() < 1e-5
            && travel.translation.y == 0.0
            && travel.translation.z == 0.0
    );
}

#[test]
fn a_loop_adds_a_whole_stride_each_time_around() {
    let (rig, pin) = walker();
    let mut player = AnimationPlayer::new(Some(0));
    let first = player.advance_travel(&rig, 3.0, pin);
    let second = player.advance_travel(&rig, 2.0, pin);
    assert!((first.translation.x - 3.0).abs() < 1e-5);
    // 3 s -> 5 s is 2 s of walking even though the clip wrapped to 1 s.
    assert!(
        (second.translation.x - 2.0).abs() < 1e-5,
        "{}",
        second.translation.x
    );
}

#[test]
fn a_once_animation_stops_carrying_at_its_end() {
    let (rig, pin) = walker();
    let mut player = AnimationPlayer::new(None);
    player.play(Some(0), Repeat::Once);
    let travel = player.advance_travel(&rig, 6.0, pin);
    assert!((travel.translation.x - 4.0).abs() < 1e-5);
    assert!(player.advance_travel(&rig, 1.0, pin).translation.x.abs() < 1e-5);
}

#[test]
fn a_crossfade_mixes_the_travel_of_both_animations() {
    let (mut rig, pin) = walker();
    rig.animations.push(Animation::default());
    let mut player = AnimationPlayer::new(Some(0));
    player.crossfade_to(Some(1), 10.0, Repeat::Loop);
    let travel = player.advance_travel(&rig, 1.0, pin);
    // The still animation is barely in yet, so most of the walk is kept.
    assert!(
        travel.translation.x > 0.9 && travel.translation.x <= 1.0,
        "{}",
        travel.translation.x
    );
}

#[test]
fn pinning_keeps_the_joint_at_rest_while_the_pose_walks() {
    let (rig, pin) = walker();
    let layers = [AnimationLayer {
        animation: Some(0),
        time: 2.0,
        weight: 1.0,
    }];
    assert!((rig.matrices_mixed(&layers)[0][0][3][0] - 2.0).abs() < 1e-5);
    assert!(rig.matrices_pinned(&layers, Some(pin))[0][0][3][0].abs() < 1e-6);
}

#[test]
fn a_pin_on_a_missing_skeleton_does_nothing() {
    let (rig, pin) = walker();
    let missing = RootMotion { skeleton: 7, ..pin };
    let mut player = AnimationPlayer::new(Some(0));
    assert_eq!(player.advance_travel(&rig, 1.0, missing).translation.x, 0.0);
}

/// A quaternion `[x, y, z, w]` turning `angle` radians about y.
fn about_y(angle: f32) -> [f32; 4] {
    [0.0, (angle / 2.0).sin(), 0.0, (angle / 2.0).cos()]
}

/// A one-joint rig whose animation, 4 seconds long, turns the joint from
/// `from` to `to` radians about y while moving it from x = 0 to x = 4.
fn turner(from: f32, to: f32, follow_yaw: bool) -> (Rig, RootMotion) {
    let (mut rig, pin) = walker();
    let turn = Keys::<4>::new(
        vec![0.0, 4.0],
        vec![about_y(from), about_y(to)],
        Interpolation::Linear,
    )
    .expect("valid keys");
    rig.animations[0].clips[0].clip = Clip::new(vec![
        Channel {
            target: pin.joint,
            track: Track::Translation(
                Keys::<3>::new(
                    vec![0.0, 4.0],
                    vec![[0.0; 3], [4.0, 0.0, 0.0]],
                    Interpolation::Linear,
                )
                .expect("valid keys"),
            ),
        },
        Channel {
            target: pin.joint,
            track: Track::Rotation(turn),
        },
    ]);
    (rig, RootMotion { follow_yaw, ..pin })
}

#[test]
fn a_turning_joint_turns_the_entity_when_asked() {
    let quarter = std::f32::consts::FRAC_PI_2;
    let (rig, pin) = turner(0.0, quarter, true);
    let mut player = AnimationPlayer::new(Some(0));
    let delta = player.advance_travel(&rig, 2.0, pin);
    assert!((delta.yaw - quarter / 2.0).abs() < 1e-4, "{}", delta.yaw);
    let (still_rig, ignoring) = turner(0.0, quarter, false);
    let mut ignorer = AnimationPlayer::new(Some(0));
    assert_eq!(ignorer.advance_travel(&still_rig, 2.0, ignoring).yaw, 0.0);
}

#[test]
fn the_turn_of_a_loop_adds_up_across_the_wrap() {
    let quarter = std::f32::consts::FRAC_PI_2;
    let (rig, pin) = turner(0.0, quarter, true);
    let mut player = AnimationPlayer::new(Some(0));
    let first = player.advance_travel(&rig, 3.0, pin).yaw;
    let second = player.advance_travel(&rig, 2.0, pin).yaw;
    // 3 s -> 5 s is 2 s of turning though the clip wrapped back to 1 s.
    assert!((first - 0.75 * quarter).abs() < 1e-4, "{first}");
    assert!((second - 0.5 * quarter).abs() < 1e-4, "{second}");
}

#[test]
fn travel_is_measured_in_the_heading_the_animation_had() {
    let quarter = std::f32::consts::FRAC_PI_2;
    // Walks along x while facing a quarter turn away from the rest heading:
    // in the entity's frame, which keeps the rest heading, that is along z.
    let (rig, pin) = turner(quarter, quarter, true);
    let mut player = AnimationPlayer::new(Some(0));
    let delta = player.advance_travel(&rig, 1.0, pin);
    assert!(delta.translation.x.abs() < 1e-4, "{:?}", delta.translation);
    assert!(
        (delta.translation.z - 1.0).abs() < 1e-4,
        "{:?}",
        delta.translation
    );
}

#[test]
fn pinning_yaw_puts_the_joint_back_to_its_rest_heading() {
    let quarter = std::f32::consts::FRAC_PI_2;
    let (rig, pin) = turner(quarter, quarter, true);
    let layers = [AnimationLayer {
        animation: Some(0),
        time: 1.0,
        weight: 1.0,
    }];
    let free = rig.matrices_mixed(&layers)[0][0];
    let pinned = rig.matrices_pinned(&layers, Some(pin))[0][0];
    // Turned a quarter, the x axis column points along -z; pinned it is x.
    assert!((free[0][2] + 1.0).abs() < 1e-4, "{free:?}");
    assert!(
        (pinned[0][0] - 1.0).abs() < 1e-4 && pinned[0][2].abs() < 1e-4,
        "{pinned:?}"
    );
}

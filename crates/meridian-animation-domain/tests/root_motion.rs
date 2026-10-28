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
    };
    (rig, pin)
}

#[test]
fn the_player_reports_how_far_the_joint_walked() {
    let (rig, pin) = walker();
    let mut player = AnimationPlayer::new(Some(0));
    let travel = player.advance_travel(&rig, 1.0, pin);
    assert!((travel.x - 1.0).abs() < 1e-5 && travel.y == 0.0 && travel.z == 0.0);
}

#[test]
fn a_loop_adds_a_whole_stride_each_time_around() {
    let (rig, pin) = walker();
    let mut player = AnimationPlayer::new(Some(0));
    let first = player.advance_travel(&rig, 3.0, pin);
    let second = player.advance_travel(&rig, 2.0, pin);
    assert!((first.x - 3.0).abs() < 1e-5);
    // 3 s -> 5 s is 2 s of walking even though the clip wrapped to 1 s.
    assert!((second.x - 2.0).abs() < 1e-5, "{}", second.x);
}

#[test]
fn a_once_animation_stops_carrying_at_its_end() {
    let (rig, pin) = walker();
    let mut player = AnimationPlayer::new(None);
    player.play(Some(0), Repeat::Once);
    let travel = player.advance_travel(&rig, 6.0, pin);
    assert!((travel.x - 4.0).abs() < 1e-5);
    assert!(player.advance_travel(&rig, 1.0, pin).x.abs() < 1e-5);
}

#[test]
fn a_crossfade_mixes_the_travel_of_both_animations() {
    let (mut rig, pin) = walker();
    rig.animations.push(Animation::default());
    let mut player = AnimationPlayer::new(Some(0));
    player.crossfade_to(Some(1), 10.0, Repeat::Loop);
    let travel = player.advance_travel(&rig, 1.0, pin);
    // The still animation is barely in yet, so most of the walk is kept.
    assert!(travel.x > 0.9 && travel.x <= 1.0, "{}", travel.x);
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
    assert_eq!(player.advance_travel(&rig, 1.0, missing).x, 0.0);
}

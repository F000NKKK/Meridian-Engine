//! Tests of rigs: sampling, mixing layers and playing with crossfades.

use meridian_animation_domain::{
    Animation, AnimationLayer, AnimationPlayer, Channel, Clip, Interpolation, Joint, JointId,
    JointMatrix, JointTransform, Keys, Repeat, Rig, Skeleton, SkeletonClip, Track, WeightKeys,
    WeightsChannel,
};

const IDENTITY: JointMatrix = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// An animation of `duration` seconds that holds the one joint at `x` and
/// the part's one morph weight at `weight`.
fn holding(x: f32, weight: f32, duration: f32) -> Animation {
    let keys = Keys::<3>::new(
        vec![0.0, duration],
        vec![[x, 0.0, 0.0]; 2],
        Interpolation::Step,
    )
    .expect("valid keys");
    let channel = Channel {
        target: JointId::from_index(0).expect("joint 0"),
        track: Track::Translation(keys),
    };
    let weights = WeightKeys::new(&[0.0], &[weight], 1, Interpolation::Step).expect("valid keys");
    Animation {
        clips: vec![SkeletonClip {
            skeleton: 0,
            clip: Clip::new(vec![channel]),
        }],
        weights: vec![WeightsChannel {
            target: 0,
            keys: weights,
        }],
    }
}

fn rig(durations: [f32; 2]) -> Rig {
    let joint = Joint {
        parent: None,
        rest: JointTransform::default(),
        inverse_bind: IDENTITY,
    };
    Rig {
        skeletons: vec![Skeleton::new(vec![joint], IDENTITY).expect("a valid skeleton")],
        animations: vec![
            holding(2.0, 0.0, durations[0]),
            holding(6.0, 1.0, durations[1]),
        ],
        rest_weights: vec![vec![0.0]],
    }
}

fn layer(animation: usize, time: f32, weight: f32) -> AnimationLayer {
    AnimationLayer {
        animation: Some(animation),
        time,
        weight,
    }
}

fn x_of(rig: &Rig, layers: &[AnimationLayer]) -> f32 {
    rig.matrices_mixed(layers)[0][0][3][0]
}

#[test]
fn an_animation_lasts_as_long_as_its_longest_track() {
    assert!((rig([3.0, 1.0]).animations[0].duration() - 3.0).abs() < f32::EPSILON);
}

#[test]
fn a_later_layer_is_blended_over_the_earlier_ones_by_its_weight() {
    let rig = rig([4.0, 4.0]);
    let at = |weight: f32| {
        let layers = [layer(0, 0.0, 1.0), layer(1, 0.0, weight)];
        (x_of(&rig, &layers), rig.weights_mixed(&layers)[0][0])
    };
    assert_eq!(at(0.0), (2.0, 0.0));
    assert_eq!(at(0.25), (3.0, 0.25));
    assert_eq!(at(1.0), (6.0, 1.0));
}

#[test]
fn one_layer_is_that_animation_and_no_layers_is_the_rest_pose() {
    let rig = rig([4.0, 4.0]);
    assert_eq!(x_of(&rig, &[layer(1, 0.0, 0.3)]), 6.0);
    assert_eq!(x_of(&rig, &[]), 0.0);
    assert_eq!(rig.weights_mixed(&[]), [vec![0.0]]);
    assert_eq!(rig.matrices(Some(0), 1.0)[0][0][3][0], 2.0);
    assert_eq!(rig.weights(Some(9), 1.0), [vec![0.0]]);
}

#[test]
fn a_player_loops_its_animation() {
    let rig = rig([2.0, 2.0]);
    let mut player = AnimationPlayer::new(Some(0));
    player.advance(&rig, 3.0);
    assert_eq!(player.layers().len(), 1);
    assert!((player.layers()[0].time - 1.0).abs() < 1e-6);
}

#[test]
fn a_crossfade_keeps_the_old_animation_running_until_it_ends() {
    let rig = rig([4.0, 4.0]);
    let mut player = AnimationPlayer::new(Some(0));
    player.advance(&rig, 1.0);
    player.crossfade_to(Some(1), 2.0, Repeat::Loop);
    assert!(player.is_fading());
    player.advance(&rig, 1.0);
    let layers = player.layers();
    assert_eq!(layers.len(), 2);
    assert_eq!((layers[0].animation, layers[0].time), (Some(0), 2.0));
    assert_eq!((layers[1].animation, layers[1].time), (Some(1), 1.0));
    assert!((layers[1].weight - 0.5).abs() < 1e-6);
    player.advance(&rig, 1.0);
    assert!(!player.is_fading());
    assert_eq!(player.layers().len(), 1);
    assert_eq!(player.animation(), Some(1));
}

#[test]
fn play_replaces_everything_at_once() {
    let rig = rig([4.0, 4.0]);
    let mut player = AnimationPlayer::new(Some(0));
    player.crossfade_to(Some(1), 5.0, Repeat::Loop);
    player.play(None, Repeat::Once);
    player.advance(&rig, 1.0);
    assert!(!player.is_fading());
    assert_eq!(player.layers()[0].animation, None);
}

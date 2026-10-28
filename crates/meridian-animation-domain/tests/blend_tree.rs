//! Tests of blend trees: weights from parameters, phase sync, root motion.

use std::sync::Arc;

use meridian_animation_domain::{
    Animation, AnimationController, AnimationGraph, AnimationPlayer, AnimationState, Blend1d,
    Blend2d, Channel, Clip, GraphError, Interpolation, Joint, JointId, JointMatrix, JointTransform,
    Keys, Mix, Motion, ParamKind, Repeat, Rig, RootMotion, Skeleton, SkeletonClip, Track,
};

const IDENTITY: JointMatrix = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// An animation of `seconds` that slides the one joint to x = `distance`.
fn sliding(seconds: f32, distance: f32) -> Animation {
    let keys = Keys::<3>::new(
        vec![0.0, seconds],
        vec![[0.0; 3], [distance, 0.0, 0.0]],
        Interpolation::Linear,
    )
    .expect("valid keys");
    Animation {
        clips: vec![SkeletonClip {
            skeleton: 0,
            clip: Clip::new(vec![Channel {
                target: JointId::from_index(0).expect("joint 0"),
                track: Track::Translation(keys),
            }]),
        }],
        weights: Vec::new(),
    }
}

/// Animation 0 stands, 1 walks 2 m in 2 s, 2 runs 3 m in 1 s, 3 and 4 strafe.
fn rig() -> Rig {
    let joint = Joint {
        parent: None,
        rest: JointTransform::default(),
        inverse_bind: IDENTITY,
    };
    Rig {
        skeletons: vec![Skeleton::new(vec![joint], IDENTITY).expect("a valid skeleton")],
        animations: vec![
            sliding(1.0, 0.0),
            sliding(2.0, 2.0),
            sliding(1.0, 3.0),
            sliding(1.0, 1.0),
            sliding(1.0, 2.0),
        ],
        rest_weights: Vec::new(),
    }
}

fn locomotion() -> (Arc<AnimationGraph>, meridian_animation_domain::ParamId) {
    let mut graph = AnimationGraph::new(None, Repeat::Loop);
    let speed = graph.add_param(ParamKind::Float);
    let blend = Blend1d::new(speed, vec![(3.0, 2), (0.0, 0), (1.0, 1)]);
    let state = graph
        .add_state(AnimationState {
            motion: Motion::Blend1d(blend),
            repeat: Repeat::Loop,
        })
        .expect("a valid state");
    graph.set_start(state).expect("its own state");
    (Arc::new(graph), speed)
}

#[test]
fn the_mix_keeps_the_heaviest_animations_and_scales_them_to_one() {
    let mix = Mix::weighted(&[(0, 1.0), (1, 3.0), (2, 0.0), (3, 2.0), (4, 2.0), (5, 0.5)]);
    let entries = mix.entries();
    assert_eq!(entries.len(), 4);
    assert_eq!(entries[0].animation, Some(1));
    assert!((entries.iter().map(|e| e.weight).sum::<f32>() - 1.0).abs() < 1e-6);
    assert_eq!(Mix::weighted(&[]).dominant(), None);
}

#[test]
fn a_1d_blend_mixes_the_two_animations_around_the_value() {
    let (graph, speed) = locomotion();
    let rig = rig();
    let mut controller = AnimationController::new(graph);
    let mut player = AnimationPlayer::new(None);
    controller.update(&mut player, &rig);
    assert!(controller.set_float(speed, 2.0));
    controller.update(&mut player, &rig);
    // The first update starts the blend state; the second reweighs it.
    let layers = player.layers();
    assert_eq!(layers.len(), 2);
    assert_eq!(
        layers.iter().map(|l| l.animation).collect::<Vec<_>>(),
        [Some(1), Some(2)]
    );
    assert!((layers[1].weight - 0.5).abs() < 1e-6);
}

#[test]
fn a_value_on_a_threshold_or_past_the_ends_plays_one_animation() {
    let (graph, speed) = locomotion();
    let rig = rig();
    for (value, animation) in [(1.0, 1), (-5.0, 0), (9.0, 2)] {
        let mut controller = AnimationController::new(graph.clone());
        let mut player = AnimationPlayer::new(None);
        controller.update(&mut player, &rig);
        controller.set_float(speed, value);
        controller.update(&mut player, &rig);
        let layers = player.layers();
        assert_eq!(layers.len(), 1, "{value}");
        assert_eq!(layers[0].animation, Some(animation), "{value}");
    }
}

#[test]
fn blended_animations_share_a_phase_so_their_steps_line_up() {
    let rig = rig();
    let mut player = AnimationPlayer::new(None);
    // Walk (2 s) and run (1 s) half and half last 1.5 s a pass.
    player.play_mix(Mix::weighted(&[(1, 0.5), (2, 0.5)]), Repeat::Loop);
    player.advance(&rig, 0.75);
    let times: Vec<f32> = player.layers().iter().map(|l| l.time).collect();
    assert!(
        (times[0] - 1.0).abs() < 1e-5 && (times[1] - 0.5).abs() < 1e-5,
        "{times:?}"
    );
    // Reweighing keeps the place in the pass.
    player.reweigh(&rig, Mix::single(Some(1)));
    assert!((player.layers()[0].time - 1.0).abs() < 1e-5);
}

#[test]
fn root_motion_of_a_blend_averages_the_travel_of_its_animations() {
    let rig = rig();
    let pin = RootMotion {
        skeleton: 0,
        joint: JointId::from_index(0).expect("joint 0"),
        follow_yaw: false,
    };
    let mut player = AnimationPlayer::new(None);
    player.play_mix(Mix::weighted(&[(1, 0.5), (2, 0.5)]), Repeat::Loop);
    // Half a pass: the walk covers 1 m, the run 1.5 m.
    let travel = player.advance_travel(&rig, 0.75, pin);
    assert!(
        (travel.translation.x - 1.25).abs() < 1e-4,
        "{:?}",
        travel.translation
    );
    // A whole pass more adds a whole stride of the average: 2.5 m.
    let again = player.advance_travel(&rig, 1.5, pin);
    assert!(
        (again.translation.x - 2.5).abs() < 1e-4,
        "{:?}",
        again.translation
    );
}

#[test]
fn a_2d_blend_plays_the_nearest_animations_by_inverse_square_distance() {
    let mut graph = AnimationGraph::new(None, Repeat::Loop);
    let (x, y) = (
        graph.add_param(ParamKind::Float),
        graph.add_param(ParamKind::Float),
    );
    let blend = Blend2d {
        x,
        y,
        entries: vec![
            ([0.0, 1.0], 1),
            ([1.0, 0.0], 2),
            ([-1.0, 0.0], 3),
            ([0.0, -1.0], 4),
        ],
    };
    let state = graph
        .add_state(AnimationState {
            motion: Motion::Blend2d(blend),
            repeat: Repeat::Loop,
        })
        .expect("a valid state");
    graph.set_start(state).expect("its own state");
    let mut controller = AnimationController::new(Arc::new(graph));
    let rig = rig();
    let mut player = AnimationPlayer::new(None);
    controller.update(&mut player, &rig);

    controller.set_float(y, 0.9);
    controller.update(&mut player, &rig);
    let layers = player.layers();
    assert!(layers.len() <= 3);
    assert_eq!(player.animation(), Some(1));

    controller.set_float(x, 1.0);
    controller.set_float(y, 0.0);
    controller.update(&mut player, &rig);
    assert_eq!(player.layers().len(), 1);
    assert_eq!(player.animation(), Some(2));
}

#[test]
fn a_blend_must_read_float_parameters_of_its_graph() {
    let mut graph = AnimationGraph::new(None, Repeat::Loop);
    let flag = graph.add_param(ParamKind::Bool);
    let blend = |param| AnimationState {
        motion: Motion::Blend1d(Blend1d::new(param, vec![(0.0, 0)])),
        repeat: Repeat::Loop,
    };
    assert_eq!(graph.add_state(blend(flag)), Err(GraphError::WrongKind(0)));
    let mut other = AnimationGraph::new(None, Repeat::Loop);
    other.add_param(ParamKind::Float);
    let foreign = other.add_param(ParamKind::Float);
    assert_eq!(
        graph.add_state(blend(foreign)),
        Err(GraphError::UnknownParam(1))
    );
}

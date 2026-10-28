//! Tests of the animation state machine: transitions, parameters, fades.

use std::sync::Arc;

use meridian_animation_domain::{
    Animation, AnimationController, AnimationGraph, AnimationPlayer, AnimationState, Condition,
    GraphError, Interpolation, ParamKind, Repeat, Rig, StateId, Transition, WeightKeys,
    WeightsChannel,
};

/// An animation that runs for `duration` seconds.
fn lasting(duration: f32) -> Animation {
    let keys =
        WeightKeys::new(&[0.0, duration], &[0.0, 1.0], 1, Interpolation::Step).expect("valid keys");
    Animation {
        clips: Vec::new(),
        weights: vec![WeightsChannel { target: 0, keys }],
    }
}

fn rig() -> Rig {
    Rig {
        animations: vec![lasting(1.0), lasting(1.0), lasting(1.0)],
        ..Rig::default()
    }
}

fn state(animation: usize, repeat: Repeat) -> AnimationState {
    AnimationState {
        animation: Some(animation),
        repeat,
    }
}

fn go(from: StateId, to: StateId, conditions: Vec<Condition>, fade: f32) -> Transition {
    Transition {
        from: Some(from),
        to,
        conditions,
        fade,
    }
}

#[test]
fn the_first_update_plays_the_start_state() {
    let graph = Arc::new(AnimationGraph::new(state(2, Repeat::Loop)));
    let mut controller = AnimationController::new(graph);
    let mut player = AnimationPlayer::new(None);
    controller.update(&mut player, &rig());
    assert_eq!(player.animation(), Some(2));
}

#[test]
fn a_float_condition_crossfades_into_the_next_state() {
    let mut graph = AnimationGraph::new(state(0, Repeat::Loop));
    let start = graph.start();
    let run = graph.add_state(state(1, Repeat::Loop));
    let speed = graph.add_param(ParamKind::Float);
    graph
        .add_transition(go(start, run, vec![Condition::Above(speed, 0.5)], 0.25))
        .unwrap();
    let mut controller = AnimationController::new(Arc::new(graph));
    let mut player = AnimationPlayer::new(None);
    controller.update(&mut player, &rig());
    assert_eq!(controller.state(), start);

    assert!(controller.set_float(speed, 1.0));
    controller.update(&mut player, &rig());
    assert_eq!(controller.state(), run);
    assert_eq!(player.animation(), Some(1));
    assert!(player.is_fading());
}

#[test]
fn a_trigger_fires_once_and_a_zero_fade_cuts() {
    let mut graph = AnimationGraph::new(state(0, Repeat::Loop));
    let start = graph.start();
    let jump = graph.add_state(state(1, Repeat::Loop));
    let press = graph.add_param(ParamKind::Trigger);
    graph
        .add_transition(go(start, jump, vec![Condition::Triggered(press)], 0.0))
        .unwrap();
    graph
        .add_transition(go(jump, start, vec![Condition::Triggered(press)], 0.0))
        .unwrap();
    let mut controller = AnimationController::new(Arc::new(graph));
    let mut player = AnimationPlayer::new(None);
    controller.update(&mut player, &rig());
    assert!(controller.trigger(press));
    controller.update(&mut player, &rig());
    assert_eq!((controller.state(), player.is_fading()), (jump, false));
    // The trigger was consumed: it does not fire the way back.
    controller.update(&mut player, &rig());
    assert_eq!(controller.state(), jump);
}

#[test]
fn finished_leaves_a_once_state_after_its_animation_ends() {
    let mut graph = AnimationGraph::new(state(0, Repeat::Once));
    let start = graph.start();
    let idle = graph.add_state(state(1, Repeat::Loop));
    graph
        .add_transition(go(start, idle, vec![Condition::Finished], 0.0))
        .unwrap();
    let rig = rig();
    let mut controller = AnimationController::new(Arc::new(graph));
    let mut player = AnimationPlayer::new(None);
    controller.update(&mut player, &rig);
    player.advance(&rig, 0.5);
    controller.update(&mut player, &rig);
    assert_eq!(controller.state(), start);
    player.advance(&rig, 0.6);
    controller.update(&mut player, &rig);
    assert_eq!(controller.state(), idle);
}

#[test]
fn a_transition_from_any_state_and_the_first_match_wins() {
    let mut graph = AnimationGraph::new(state(0, Repeat::Loop));
    let a = graph.add_state(state(1, Repeat::Loop));
    let b = graph.add_state(state(2, Repeat::Loop));
    for to in [a, b] {
        graph
            .add_transition(Transition {
                from: None,
                to,
                conditions: Vec::new(),
                fade: 0.0,
            })
            .unwrap();
    }
    let mut controller = AnimationController::new(Arc::new(graph));
    let mut player = AnimationPlayer::new(None);
    controller.update(&mut player, &rig());
    assert_eq!(controller.state(), a);
}

#[test]
fn a_graph_refuses_foreign_ids_and_wrong_kinds() {
    let mut other = AnimationGraph::new(state(0, Repeat::Loop));
    let foreign_state = other.add_state(state(0, Repeat::Loop));
    let foreign_param = other.add_param(ParamKind::Float);
    let mut graph = AnimationGraph::new(state(0, Repeat::Loop));
    let start = graph.start();
    assert_eq!(
        graph.add_transition(go(start, foreign_state, Vec::new(), 0.0)),
        Err(GraphError::UnknownState(1))
    );
    assert_eq!(
        graph.add_transition(go(
            start,
            start,
            vec![Condition::Above(foreign_param, 1.0)],
            0.0
        )),
        Err(GraphError::UnknownParam(0))
    );
    let flag = graph.add_param(ParamKind::Bool);
    assert_eq!(
        graph.add_transition(go(start, start, vec![Condition::Above(flag, 1.0)], 0.0)),
        Err(GraphError::WrongKind(0))
    );
}

#[test]
fn setting_a_parameter_of_the_wrong_kind_is_refused() {
    let mut graph = AnimationGraph::new(state(0, Repeat::Loop));
    let flag = graph.add_param(ParamKind::Bool);
    let mut controller = AnimationController::new(Arc::new(graph));
    assert!(!controller.set_float(flag, 1.0));
    assert!(controller.set_bool(flag, true));
}

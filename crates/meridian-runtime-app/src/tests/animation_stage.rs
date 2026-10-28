//! Tests of the animation stage.

use std::sync::Arc;

use meridian_animation_domain::{
    Animation, AnimationController, AnimationGraph, AnimationPlayer, AnimationState,
    AnimationTarget, Animator, Channel, Clip, Condition, Interpolation, Joint, JointId,
    JointTransform, Keys, Motion, ParamKind, Repeat, Rig, RootMotion, Skeleton, SkeletonClip,
    Track, Transition, WeightKeys, WeightsChannel,
};
use meridian_ecs::{Entity, Transform, World};
use meridian_gac::{Motor3, Rotor, Vec3};
use meridian_rendering_domain::{JointPalette, MorphWeights};

use crate::{AnimationStage, FrameDelta, Shared, Stage};

const IDENTITY: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// One joint that slides from x = 0 to x = 4 over 4 seconds, and one morph
/// weight that rises from 0 to 1 over the same time.
fn rig() -> Arc<Rig> {
    let slide = Keys::<3>::new(
        vec![0.0, 4.0],
        vec![[0.0; 3], [4.0, 0.0, 0.0]],
        Interpolation::Linear,
    )
    .expect("valid keys");
    let rise =
        WeightKeys::new(&[0.0, 4.0], &[0.0, 1.0], 1, Interpolation::Linear).expect("valid keys");
    let joint = Joint {
        parent: None,
        rest: JointTransform::default(),
        inverse_bind: IDENTITY,
    };
    Arc::new(Rig {
        skeletons: vec![Skeleton::new(vec![joint], IDENTITY).expect("a valid skeleton")],
        animations: vec![Animation {
            clips: vec![SkeletonClip {
                skeleton: 0,
                clip: Clip::new(vec![Channel {
                    target: JointId::from_index(0).expect("joint 0"),
                    track: Track::Translation(slide),
                }]),
            }],
            weights: vec![WeightsChannel {
                target: 0,
                keys: rise,
            }],
        }],
        rest_weights: vec![vec![0.0]],
    })
}

fn spawn_animator(world: &mut World, rig: &Arc<Rig>) -> Entity {
    let entity = world.spawn();
    world.insert(
        entity,
        Animator {
            rig: Arc::clone(rig),
            player: AnimationPlayer::new(Some(0)),
        },
    );
    entity
}

fn spawn_target(world: &mut World, animator: Entity, skeleton: Option<usize>) -> Entity {
    let entity = world.spawn();
    world.insert(
        entity,
        AnimationTarget {
            animator,
            skeleton,
            part: 0,
        },
    );
    entity
}

fn stage(world: &Shared<World>, seconds: f32) -> AnimationStage {
    AnimationStage::new(world.clone(), Shared::new(FrameDelta(seconds)))
}

#[test]
fn a_target_receives_the_pose_of_its_animator_after_each_frame() {
    let rig = rig();
    let world = Shared::new(World::new());
    let (animator, target) = {
        let mut world = world.lock();
        let animator = spawn_animator(&mut world, &rig);
        (animator, spawn_target(&mut world, animator, Some(0)))
    };
    let mut stage = stage(&world, 1.0);
    stage.run();
    stage.run();
    let guard = world.lock();
    let palette = guard.get::<JointPalette>(target).expect("a palette");
    assert!((palette.matrices()[0][3][0] - 2.0).abs() < 1e-5);
    let weights = guard.get::<MorphWeights>(target).expect("weights");
    assert!((weights.0[0] - 0.5).abs() < 1e-6);
    assert!(guard.get::<JointPalette>(animator).is_none());
}

#[test]
fn a_rigid_target_gets_weights_but_no_palette_and_an_orphan_gets_nothing() {
    let rig = rig();
    let world = Shared::new(World::new());
    let (rigid, orphan) = {
        let mut world = world.lock();
        let animator = spawn_animator(&mut world, &rig);
        let rigid = spawn_target(&mut world, animator, None);
        let gone = world.spawn();
        world.despawn(gone);
        (rigid, spawn_target(&mut world, gone, Some(0)))
    };
    stage(&world, 1.0).run();
    let guard = world.lock();
    assert!(guard.get::<JointPalette>(rigid).is_none());
    assert!(guard.get::<MorphWeights>(rigid).is_some());
    assert!(guard.get::<JointPalette>(orphan).is_none());
    assert!(guard.get::<MorphWeights>(orphan).is_none());
}

#[test]
fn animators_over_one_rig_play_independently() {
    let rig = rig();
    let world = Shared::new(World::new());
    let (fast, slow) = {
        let mut world = world.lock();
        let fast = spawn_animator(&mut world, &rig);
        let slow = spawn_animator(&mut world, &rig);
        (
            (fast, spawn_target(&mut world, fast, Some(0))),
            (slow, spawn_target(&mut world, slow, Some(0))),
        )
    };
    stage(&world, 1.0).run();
    world
        .lock()
        .get_mut::<Animator>(slow.0)
        .expect("an animator")
        .player
        .play(None, Repeat::Loop);
    stage(&world, 1.0).run();
    let world = world.lock();
    let x = |entity| {
        world
            .get::<JointPalette>(entity)
            .expect("a palette")
            .matrices()[0][3][0]
    };
    assert!((x(fast.1) - 2.0).abs() < 1e-5, "{}", x(fast.1));
    assert!(x(slow.1).abs() < 1e-5, "{}", x(slow.1));
}

#[test]
fn a_controller_switches_the_animation_when_gameplay_sets_a_parameter() {
    let rig = rig();
    let world = Shared::new(World::new());
    let mut graph = AnimationGraph::new(None, Repeat::Loop);
    let idle = graph.start();
    let moving = graph
        .add_state(AnimationState {
            motion: Motion::Clip(Some(0)),
            repeat: Repeat::Loop,
        })
        .expect("a valid state");
    let walking = graph.add_param(ParamKind::Bool);
    graph
        .add_transition(Transition {
            from: Some(idle),
            to: moving,
            conditions: vec![Condition::Is(walking, true)],
            fade: 0.0,
        })
        .expect("a valid transition");
    let animator = {
        let mut world = world.lock();
        let animator = spawn_animator(&mut world, &rig);
        world.insert(animator, AnimationController::new(Arc::new(graph)));
        animator
    };
    stage(&world, 1.0).run();
    let player = |shared: &Shared<World>| shared.lock().get::<Animator>(animator).unwrap().player;
    assert_eq!(player(&world).animation(), None);
    world
        .lock()
        .get_mut::<AnimationController>(animator)
        .unwrap()
        .set_bool(walking, true);
    stage(&world, 1.0).run();
    assert_eq!(player(&world).animation(), Some(0));
}

#[test]
fn root_motion_moves_the_entity_in_its_own_frame_and_pins_the_pose() {
    let rig = rig();
    let world = Shared::new(World::new());
    // Turned a quarter about y, so the model's +x is the world's -z.
    let facing = Rotor::from_axis_angle(Vec3::new(0.0, 1.0, 0.0), std::f32::consts::FRAC_PI_2);
    let (animator, target) = {
        let mut world = world.lock();
        let animator = spawn_animator(&mut world, &rig);
        world.insert(
            animator,
            RootMotion {
                skeleton: 0,
                joint: JointId::from_index(0).expect("joint 0"),
                follow_yaw: false,
            },
        );
        world.insert(
            animator,
            Transform::from_motor(Motor3::from_rotation_translation(facing, Vec3::ZERO)),
        );
        (animator, spawn_target(&mut world, animator, Some(0)))
    };
    stage(&world, 2.0).run();
    let guard = world.lock();
    let moved = guard.get::<Transform>(animator).expect("a transform").motor;
    let at = moved.transform_point(Vec3::ZERO);
    assert!(
        (at.x.abs() + at.y.abs()) < 1e-4 && (at.z.abs() - 2.0).abs() < 1e-4,
        "{at:?}"
    );
    let palette = guard.get::<JointPalette>(target).expect("a palette");
    assert!(palette.matrices()[0][3][0].abs() < 1e-5);
}

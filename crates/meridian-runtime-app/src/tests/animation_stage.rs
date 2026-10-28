//! Tests of the animation stage.

use std::sync::Arc;

use meridian_animation_domain::{
    Animation, AnimationPlayer, AnimationTarget, Animator, Channel, Clip, Interpolation, Joint,
    JointId, JointTransform, Keys, Repeat, Rig, Skeleton, SkeletonClip, Track, WeightKeys,
    WeightsChannel,
};
use meridian_ecs::{Entity, World};
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

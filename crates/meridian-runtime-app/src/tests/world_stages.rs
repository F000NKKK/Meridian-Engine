//! Tests of the animation and transform stages registered together.

use std::sync::Arc;

use meridian_animation_domain::{
    Animation, AnimationPlayer, Animator, Channel, Clip, Interpolation, Joint, JointId,
    JointMatrix, JointTransform, Keys, Rig, RootMotion, Skeleton, SkeletonClip, Track,
};
use meridian_ecs::{GlobalTransform, Parent, Transform, World};
use meridian_gac::Vec3;

use crate::testing::InlineExecutor;
use crate::{FrameDelta, Runtime, Shared, add_world_stages};

const IDENTITY: JointMatrix = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

/// A rig whose one joint walks 4 units along x in 4 seconds.
fn walker() -> (Arc<Rig>, RootMotion) {
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
    (Arc::new(rig), pin)
}

#[test]
fn children_follow_root_motion_in_the_same_tick() {
    let (rig, pin) = walker();
    let world = Shared::new(World::new());
    let child = {
        let mut world = world.lock();
        let root = world.spawn();
        world.insert(root, Transform::default());
        world.insert(root, pin);
        world.insert(
            root,
            Animator {
                rig,
                player: AnimationPlayer::new(Some(0)),
            },
        );
        let child = world.spawn();
        world.insert(child, Transform::default());
        world.insert(child, Parent(root));
        child
    };
    let delta = Shared::new(FrameDelta(2.0));
    let mut runtime = Runtime::new(Arc::new(InlineExecutor));
    add_world_stages(&mut runtime, &world, &delta, &[]);

    runtime.tick();

    let guard = world.lock();
    let at = guard
        .get::<GlobalTransform>(child)
        .expect("resolved")
        .motor
        .transform_point(Vec3::ZERO);
    assert!((at.x - 2.0).abs() < 1e-4, "{at:?}");
}

#[test]
fn the_transform_stage_depends_on_the_animation_stage() {
    let world = Shared::new(World::new());
    let delta = Shared::new(FrameDelta(0.0));
    let mut runtime = Runtime::new(Arc::new(InlineExecutor));
    let stages = add_world_stages(&mut runtime, &world, &delta, &[]);
    assert_ne!(stages.animation, stages.transform);
    // Naming the transform stage before the one it depends on is refused.
    let refused = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        runtime.tick_only(&[stages.transform, stages.animation]);
    }));
    assert!(refused.is_err());
}

/// `rig` with its one animation turning the joint a quarter turn about y
/// over 4 seconds while it walks 4 units along x.
fn turning(rig: &Rig, pin: RootMotion) -> Rig {
    let quarter = std::f32::consts::FRAC_PI_2;
    let mut turning = rig.clone();
    let half = quarter / 2.0;
    let turn = Keys::<4>::new(
        vec![0.0, 4.0],
        vec![[0.0, 0.0, 0.0, 1.0], [0.0, half.sin(), 0.0, half.cos()]],
        Interpolation::Linear,
    )
    .expect("valid keys");
    let walk = Keys::<3>::new(
        vec![0.0, 4.0],
        vec![[0.0; 3], [4.0, 0.0, 0.0]],
        Interpolation::Linear,
    )
    .expect("valid keys");
    turning.animations[0].clips[0].clip = Clip::new(vec![
        Channel {
            target: pin.joint,
            track: Track::Translation(walk),
        },
        Channel {
            target: pin.joint,
            track: Track::Rotation(turn),
        },
    ]);
    turning
}

#[test]
fn root_motion_that_follows_yaw_turns_the_entity_and_steps_before_turning() {
    let (rig, pin) = walker();
    let world = Shared::new(World::new());
    let root = {
        let mut world = world.lock();
        let root = world.spawn();
        world.insert(root, Transform::default());
        world.insert(
            root,
            RootMotion {
                follow_yaw: true,
                ..pin
            },
        );
        world.insert(
            root,
            Animator {
                rig: Arc::new(turning(&rig, pin)),
                player: AnimationPlayer::new(Some(0)),
            },
        );
        root
    };
    let delta = Shared::new(FrameDelta(2.0));
    let mut runtime = Runtime::new(Arc::new(InlineExecutor));
    add_world_stages(&mut runtime, &world, &delta, &[]);

    runtime.tick();

    let guard = world.lock();
    let motor = guard.get::<Transform>(root).expect("a transform").motor;
    let origin = motor.transform_point(Vec3::ZERO);
    assert!(
        (origin.x - 2.0).abs() < 1e-4 && origin.z.abs() < 1e-4,
        "{origin:?}"
    );
    let facing = motor.transform_vector(Vec3::new(1.0, 0.0, 0.0));
    // Half of the quarter turn is done after 2 s of 4.
    let angle = std::f32::consts::FRAC_PI_4;
    assert!((facing.x - angle.cos()).abs() < 1e-4, "{facing:?}");
    assert!((facing.z + angle.sin()).abs() < 1e-4, "{facing:?}");
}

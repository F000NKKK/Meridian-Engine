//! Tests of spawning a loaded model into a world.

use std::sync::Arc;

use meridian_animation_domain::{Animation, AnimationTarget, Animator, Rig};
use meridian_asset_domain::{ModelAsset, ModelPart};
use meridian_ecs::{Parent, Transform, World, propagate_transforms};
use meridian_gac::{Motor3, Vec3};
use meridian_rendering_app::testing::FakeTextureStore;
use meridian_rendering_domain::{
    Camera, MaterialRegistry, MeshRegistry, MeshRenderer, extract_scene3d,
};

use super::{LoadedModel, register, spawn};

fn model(rig: Rig, skeleton: Option<usize>) -> LoadedModel {
    let part = ModelPart {
        positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        indices: vec![0, 1, 2],
        ..ModelPart::default()
    };
    let asset = ModelAsset {
        parts: vec![part.clone(), part],
        ..ModelAsset::default()
    };
    let mut loaded = register(
        asset,
        &mut MeshRegistry::new(),
        &mut FakeTextureStore::new(),
    )
    .unwrap();
    for piece in &mut loaded.pieces {
        piece.skeleton = skeleton;
    }
    loaded.rig = Arc::new(rig);
    loaded
}

#[test]
fn a_static_model_is_a_root_with_drawn_children_and_no_animator() {
    let mut world = World::new();
    let at = Transform {
        motor: Motor3::translation(Vec3::new(3.0, 0.0, 0.0)),
    };
    let instance = spawn(
        &mut world,
        &model(Rig::default(), None),
        &mut MaterialRegistry::new(),
        at,
    );
    assert!(world.get::<Animator>(instance.root).is_none());
    for piece in &instance.pieces {
        assert_eq!(
            world.get::<Parent>(*piece).map(|parent| parent.0),
            Some(instance.root)
        );
        assert!(world.get::<MeshRenderer>(*piece).is_some());
        assert!(world.get::<AnimationTarget>(*piece).is_none());
    }
    propagate_transforms(&mut world);
    let scene = extract_scene3d(&world, Camera::default());
    assert_eq!(scene.renderables.len(), 2);
    assert!(scene.renderables.iter().all(|r| r.frame == at.motor));
}

#[test]
fn an_animated_model_gets_an_animator_and_a_target_per_piece() {
    let rig = Rig {
        animations: vec![Animation::default()],
        ..Rig::default()
    };
    let mut world = World::new();
    let instance = spawn(
        &mut world,
        &model(rig, Some(0)),
        &mut MaterialRegistry::new(),
        Transform::default(),
    );
    let animator = world.get::<Animator>(instance.root).expect("animator");
    assert_eq!(animator.player.animation(), None);
    let target = world
        .get::<AnimationTarget>(instance.pieces[1])
        .expect("target");
    assert_eq!(target.animator, instance.root);
    assert_eq!((target.skeleton, target.part), (Some(0), 1));
}

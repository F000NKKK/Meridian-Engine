//! Putting a loaded model into an ECS world (ADR 048).

use meridian_animation_domain::{AnimationPlayer, AnimationTarget, Animator};
use meridian_ecs::{Entity, Parent, Transform, World};
use meridian_rendering_domain::{MaterialRegistry, MeshRenderer};

use super::LoadedModel;

/// The entities [`spawn`] made for one model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelInstance {
    /// The entity that places the whole model. Move its `Transform` to move
    /// every piece; it holds the [`Animator`] when the model has a rig.
    pub root: Entity,
    /// One entity per piece, in the order of [`LoadedModel::pieces`], each a
    /// child of `root`.
    pub pieces: Vec<Entity>,
}

/// Spawns `model` into `world`: a root entity at `at`, and under it one
/// entity per piece with the piece's [`MeshRenderer`]. A model with
/// skeletons or animations also gets an [`Animator`] on the root (playing
/// nothing until its player is told to) and an [`AnimationTarget`] on each
/// piece, so the animation stage poses it.
///
/// Pieces sit at the root's frame through the hierarchy, so
/// `extract_scene3d` needs the transform stage to have run.
pub(crate) fn spawn(
    world: &mut World,
    model: &LoadedModel,
    materials: &mut MaterialRegistry,
    at: Transform,
) -> ModelInstance {
    let root = world.spawn();
    world.insert(root, at);
    let animated = !model.rig.skeletons.is_empty() || !model.rig.animations.is_empty();
    if animated {
        world.insert(
            root,
            Animator {
                rig: model.rig.clone(),
                player: AnimationPlayer::new(None),
            },
        );
    }
    let pieces = model
        .pieces
        .iter()
        .enumerate()
        .map(|(part, piece)| {
            let entity = world.spawn();
            world.insert(entity, Transform::default());
            world.insert(entity, Parent(root));
            world.insert(
                entity,
                MeshRenderer {
                    mesh: piece.mesh,
                    material: materials.register(piece.material),
                    billboard: false,
                },
            );
            if animated {
                world.insert(
                    entity,
                    AnimationTarget {
                        animator: root,
                        skeleton: piece.skeleton,
                        part,
                    },
                );
            }
            entity
        })
        .collect();
    ModelInstance { root, pieces }
}

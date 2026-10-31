//! Binding the asset references a scene file holds to the handles the
//! renderer draws with.

use meridian_data_substrate::{AssetKey, MeshReference};
use meridian_ecs::{Component, Entity, World, propagate_transforms};
use meridian_rendering_domain::{MaterialHandle, MeshHandle, MeshRenderer, TextureHandle};

use crate::GraphicsSession;
use crate::editor_renderer::EditorRenderer;
use crate::model_loading::{LoadedModel, attach};

/// Turns an `@asset:` path into something drawable: the application's
/// answer to "what is `meshes/hero.obj`?". The path is data from a file, so
/// it is looked up once here and the renderer only ever sees handles.
pub trait AssetBinder {
    /// The mesh `key` names, registered with `session`, or `None` when it
    /// names nothing this binder knows.
    fn mesh(&mut self, session: &mut GraphicsSession, key: &AssetKey) -> Option<MeshHandle>;

    /// The model (a glTF file with its pieces, materials and rig) `key` names,
    /// registered with `session`, or `None` when it is not a model. Asked
    /// before [`mesh`](Self::mesh): a reference to a model draws the model's own
    /// pieces and materials, and its `material` is not looked up. A binder
    /// with no models keeps this default.
    fn model(&mut self, _session: &mut GraphicsSession, _key: &AssetKey) -> Option<LoadedModel> {
        None
    }

    /// The material `key` names, registered with `session`, or `None`.
    fn material(&mut self, session: &mut GraphicsSession, key: &AssetKey)
    -> Option<MaterialHandle>;

    /// The texture `key` names, uploaded as `slot` needs (colour data as
    /// sRGB, the rest linear), or `None`. A binder with no textures keeps
    /// this default.
    fn texture(
        &mut self,
        _session: &mut GraphicsSession,
        _key: &AssetKey,
        _slot: TextureSlot,
    ) -> Option<TextureHandle> {
        None
    }
}

/// Which input of a material a texture feeds, and so how it is uploaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextureSlot {
    /// Base colour (sRGB data).
    Albedo,
    /// Roughness in green, metalness in blue (linear data).
    MetallicRoughness,
    /// Tangent-space normals (linear data).
    Normal,
    /// Occlusion in red (linear data).
    Occlusion,
    /// Emitted colour (sRGB data).
    Emissive,
}

/// The piece entities made under an entity whose reference named a model:
/// each is a child of that entity, in the order of the model's pieces. Marks
/// the entity as bound, so it is not bound again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModelPieces(pub Vec<Entity>);

impl Component for ModelPieces {}

/// A reference no binder could resolve, with the entity that holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnboundReference {
    /// The entity whose `mesh_renderer` could not be bound.
    pub entity: Entity,
    /// The path that resolved to nothing.
    pub missing: AssetKey,
}

impl EditorRenderer {
    /// Gives every entity that has a [`MeshReference`] but no `MeshRenderer`
    /// the handles `binder` resolves its references to, and returns the
    /// references it could not resolve (those entities stay undrawn). A
    /// reference the binder answers with a model gets one child entity per
    /// piece (see [`ModelPieces`]); a world with models needs
    /// `propagate_transforms` after its owners move.
    pub fn bind_references(
        &mut self,
        world: &mut World,
        binder: &mut dyn AssetBinder,
    ) -> Vec<UnboundReference> {
        bind_references(self.session_mut(), world, binder)
    }
}

/// Binds scene asset references using an application's existing graphics session.
pub fn bind_references(
    session: &mut GraphicsSession,
    world: &mut World,
    binder: &mut dyn AssetBinder,
) -> Vec<UnboundReference> {
    let pending: Vec<(Entity, MeshReference)> = world
        .query::<MeshReference>()
        .filter(|(entity, _)| {
            world.get::<MeshRenderer>(*entity).is_none()
                && world.get::<ModelPieces>(*entity).is_none()
        })
        .map(|(entity, reference)| (entity, reference.clone()))
        .collect();
    let mut unbound = Vec::new();
    let mut placed_models = false;
    for (entity, reference) in pending {
        if let Some(model) = binder.model(session, &reference.mesh.id) {
            let pieces = attach(world, entity, &model, &mut session.render.materials);
            world.insert(entity, ModelPieces(pieces));
            placed_models = true;
            continue;
        }
        let mesh = binder.mesh(session, &reference.mesh.id);
        let material = binder.material(session, &reference.material.id);
        match (mesh, material) {
            (Some(mesh), Some(material)) => {
                world.insert(
                    entity,
                    MeshRenderer {
                        mesh,
                        material,
                        billboard: reference.billboard,
                    },
                );
            }
            (None, _) => unbound.push(UnboundReference {
                entity,
                missing: reference.mesh.id,
            }),
            (_, None) => unbound.push(UnboundReference {
                entity,
                missing: reference.material.id,
            }),
        }
    }
    if placed_models {
        propagate_transforms(world);
    }
    unbound
}

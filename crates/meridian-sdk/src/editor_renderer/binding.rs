//! Binding the asset references a scene file holds to the handles the
//! renderer draws with.

use meridian_data_substrate::{AssetKey, MeshReference};
use meridian_ecs::{Entity, World};
use meridian_rendering_domain::{MaterialHandle, MeshHandle, MeshRenderer, TextureHandle};

use super::EditorRenderer;
use crate::GraphicsSession;

/// Turns an `@asset:` path into something drawable: the application's
/// answer to "what is `meshes/hero.obj`?". The path is data from a file, so
/// it is looked up once here and the renderer only ever sees handles.
pub trait AssetBinder {
    /// The mesh `key` names, registered with `session`, or `None` when it
    /// names nothing this binder knows.
    fn mesh(&mut self, session: &mut GraphicsSession, key: &AssetKey) -> Option<MeshHandle>;

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
    /// references it could not resolve (those entities stay undrawn).
    pub fn bind_references(
        &mut self,
        world: &mut World,
        binder: &mut dyn AssetBinder,
    ) -> Vec<UnboundReference> {
        let pending: Vec<(Entity, MeshReference)> = world
            .query::<MeshReference>()
            .filter(|(entity, _)| world.get::<MeshRenderer>(*entity).is_none())
            .map(|(entity, reference)| (entity, reference.clone()))
            .collect();
        let session = self.session_mut();
        let mut unbound = Vec::new();
        for (entity, reference) in pending {
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
        unbound
    }
}

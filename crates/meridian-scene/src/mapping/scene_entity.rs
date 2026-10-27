//! [`SceneEntity`]: one entity of a scene, as its document describes it.

use meridian_ecs_core::Entity;

use crate::{DisplayName, EntityKey, KeptComponent, PrefabPath};

/// One entity of a loaded (or to-be-saved) scene: its document key, its
/// runtime handle, and what the document says of it besides components.
/// Runtime `Entity` handles are process-local and never written.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneEntity {
    /// Stable within the document.
    pub key: EntityKey,
    /// The entity it was spawned as.
    pub entity: Entity,
    /// A name for people.
    pub name: Option<DisplayName>,
    /// The prefab it starts from (checked by the caller's resolver; not
    /// instanced yet).
    pub prefab: Option<PrefabPath>,
    /// Its parent in the same document. A [`crate::Parent`] component, when
    /// the entity has one, wins over this on save.
    pub parent: Option<EntityKey>,
    /// Components the registry doesn't know, kept as written when it
    /// [keeps them](crate::UnknownComponents::Keep).
    pub kept: Vec<KeptComponent>,
}

impl SceneEntity {
    /// `entity` under `key`, with nothing else said of it.
    #[must_use]
    pub fn new(key: EntityKey, entity: Entity) -> Self {
        SceneEntity {
            key,
            entity,
            name: None,
            prefab: None,
            parent: None,
            kept: Vec::new(),
        }
    }
}

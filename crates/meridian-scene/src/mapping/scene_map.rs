//! [`SceneMap`]: a scene's document keys and their entities.

use meridian_ecs_core::Entity;

use super::{DuplicateKey, SceneEntity};
use crate::{DisplayName, EntityKey};

/// A scene's entities in document order, each under its document key —
/// what `load_scene` returns and `save_scene` writes. Keys are unique.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneMap {
    name: Option<DisplayName>,
    entities: Vec<SceneEntity>,
}

impl SceneMap {
    /// An empty scene called `name`.
    #[must_use]
    pub fn new(name: Option<DisplayName>) -> Self {
        SceneMap {
            name,
            entities: Vec::new(),
        }
    }

    /// The scene's name for people.
    #[must_use]
    pub fn name(&self) -> Option<&DisplayName> {
        self.name.as_ref()
    }

    /// Its entities, in document order.
    #[must_use]
    pub fn entities(&self) -> &[SceneEntity] {
        &self.entities
    }

    /// The entry under `key`.
    #[must_use]
    pub fn get(&self, key: &EntityKey) -> Option<&SceneEntity> {
        self.entities.iter().find(|e| &e.key == key)
    }

    /// The entity under `key`.
    #[must_use]
    pub fn entity(&self, key: &EntityKey) -> Option<Entity> {
        self.get(key).map(|e| e.entity)
    }

    /// The key `entity` is under.
    #[must_use]
    pub fn key_of(&self, entity: Entity) -> Option<&EntityKey> {
        self.entities
            .iter()
            .find(|e| e.entity == entity)
            .map(|e| &e.key)
    }

    /// The parent the document gives `entity`.
    #[must_use]
    pub fn parent_of(&self, entity: Entity) -> Option<Entity> {
        let e = self.entities.iter().find(|e| e.entity == entity)?;
        self.entity(e.parent.as_ref()?)
    }

    /// Adds `entry` at the end.
    ///
    /// # Errors
    ///
    /// [`DuplicateKey`] when its key is already in the scene.
    pub fn push(&mut self, entry: SceneEntity) -> Result<&mut SceneEntity, DuplicateKey> {
        if self.get(&entry.key).is_some() {
            return Err(DuplicateKey { key: entry.key });
        }
        self.entities.push(entry);
        let last = self.entities.len() - 1;
        Ok(&mut self.entities[last])
    }
}

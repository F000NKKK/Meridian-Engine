//! [`SceneWriter`]: a scene's entities as `.mscene` text.

use meridian_ecs_core::World;
use substrate_data::{Id, Map, Schema, Value, text};

use crate::document_keys::{COMPONENTS, ENTITIES, FORMAT_KEY, ID, NAME, PARENT, PREFAB};
use crate::{
    ComponentRegistry, EntityKey, EntityKind, FORMAT, Parent, PrefabKind, SceneEntity, SceneMap,
};

/// Writes the entities of a [`SceneMap`] that are alive in a `World`: each
/// registered component it has, then its kept unknown ones. Floats are
/// written so they read back bit-exactly.
pub(crate) struct SceneWriter<'a> {
    registry: &'a ComponentRegistry,
    world: &'a World,
    scene: &'a SceneMap,
}

impl<'a> SceneWriter<'a> {
    pub(crate) fn new(
        registry: &'a ComponentRegistry,
        world: &'a World,
        scene: &'a SceneMap,
    ) -> Self {
        SceneWriter {
            registry,
            world,
            scene,
        }
    }

    /// The document, as text.
    pub(crate) fn text(&self) -> String {
        let mut doc = Map::new();
        doc.insert(FORMAT_KEY, Value::Int(i64::from(FORMAT)));
        if let Some(name) = self.scene.name() {
            doc.insert(NAME, name.write());
        }
        let alive = self
            .scene
            .entities()
            .iter()
            .filter(|e| self.world.is_alive(e.entity));
        doc.insert(
            ENTITIES,
            Value::List(alive.map(|e| self.entity(e)).collect()),
        );
        text::to_string_pretty(&Value::Map(doc))
    }

    fn entity(&self, e: &SceneEntity) -> Value {
        let mut m = Map::new();
        m.insert(ID, e.key.write());
        if let Some(name) = &e.name {
            m.insert(NAME, name.write());
        }
        if let Some(prefab) = &e.prefab {
            m.insert(PREFAB, Id::<PrefabKind>::new(prefab.clone()).write());
        }
        if let Some(parent) = self.parent(e) {
            m.insert(PARENT, Id::<EntityKind>::new(parent).write());
        }
        m.insert(COMPONENTS, self.components(e));
        Value::Map(m)
    }

    /// Its parent's key: its [`Parent`] component's, else the document's.
    fn parent(&self, e: &SceneEntity) -> Option<EntityKey> {
        match self.world.get::<Parent>(e.entity) {
            Some(Parent(p)) => self.scene.key_of(*p).cloned(),
            None => e.parent.clone(),
        }
    }

    fn components(&self, e: &SceneEntity) -> Value {
        let mut m = Map::new();
        for entry in self.registry.entries() {
            if let Some(v) = (entry.write)(self.world, e.entity) {
                m.insert(entry.name.as_str(), v);
            }
        }
        for kept in e
            .kept
            .iter()
            .filter(|k| self.registry.entry(&k.name).is_none())
        {
            m.insert(kept.name.as_str(), kept.value.clone());
        }
        Value::Map(m)
    }
}

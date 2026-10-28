//! [`SceneWriter`]: a scene's entities as `.mscene` text.

use std::collections::HashMap;

use meridian_ecs::World;
use substrate_data::{Id, Map, Schema, Value, binary, text};

use crate::document_keys::{COMPONENTS, ENTITIES, FORMAT_KEY, ID, NAME, OVERRIDES, PARENT, PREFAB};
use crate::document_kind::DocumentKind;
use crate::overrides::{ComponentDocs, InstanceOrigin};
use crate::read::ReadContext;
use crate::{
    ComponentRegistry, EntityKey, EntityKind, FORMAT, NoResolver, Parent, PrefabInstance,
    PrefabKind, PrefabSource, SceneEntity, SceneMap, SceneSaveError,
};

/// Writes the entities of a [`SceneMap`] that are alive in a `World`: each
/// registered component it has, then its kept unknown ones. Floats are
/// written so they read back bit-exactly.
///
/// An instance (an entity that says `prefab`) is written as it is, with
/// all its current components, unless the writer was given the prefabs
/// ([`SceneWriter::against_prefabs`]): then it is written as what changed
/// from its prefab, the edits to the entities the prefab made included.
pub(crate) struct SceneWriter<'a> {
    registry: &'a ComponentRegistry,
    world: &'a World,
    scene: &'a SceneMap,
    bases: HashMap<EntityKey, InstanceOrigin>,
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
            bases: HashMap::new(),
        }
    }

    /// This writer, reading `prefabs` for what each instance of `scene`
    /// starts from, so that it writes only what changed.
    ///
    /// # Errors
    ///
    /// [`SceneSaveError`] when a prefab is missing or does not load.
    pub(crate) fn against_prefabs(
        mut self,
        prefabs: &mut dyn PrefabSource,
    ) -> Result<Self, SceneSaveError> {
        let roots = self.scene.entities().iter().filter(|e| self.is_instance(e));
        let (mut bases, mut resolver) = (HashMap::new(), NoResolver);
        for e in roots {
            let Some(prefab) = e.prefab.clone() else {
                continue;
            };
            let instance = PrefabInstance::new(e.key.clone(), prefab);
            let mut cx = ReadContext::new(self.registry, &mut resolver).instancing(&mut *prefabs);
            let draft = cx
                .instanced(&instance)
                .map_err(|error| SceneSaveError::Prefab {
                    instance: e.key.clone(),
                    prefab: instance.prefab.clone(),
                    error,
                })?;
            bases.insert(e.key.clone(), InstanceOrigin::of_draft(&draft));
        }
        self.bases = bases;
        Ok(self)
    }

    /// Whether `e` is alive, says `prefab`, and is in the document itself.
    fn is_instance(&self, e: &SceneEntity) -> bool {
        self.world.is_alive(e.entity) && e.prefab.is_some() && e.generated_by.is_none()
    }

    /// The document, as text.
    pub(crate) fn text(&self, kind: DocumentKind) -> String {
        text::to_string_pretty(&self.document(kind))
    }

    /// The document, in the `SDAT` binary form: the canonical encoding of
    /// its value, so lists of numbers are packed and entities (maps of one
    /// shape) share their keys.
    pub(crate) fn bytes(&self, kind: DocumentKind) -> Vec<u8> {
        binary::encode(&self.document(kind))
    }

    /// The document, as a value.
    pub(crate) fn document(&self, kind: DocumentKind) -> Value {
        let mut doc = Map::new();
        doc.insert(FORMAT_KEY, Value::Int(i64::from(FORMAT)));
        match (kind, self.scene.name()) {
            (DocumentKind::Scene, Some(name)) => doc.insert(NAME, name.write()),
            _ => None,
        };
        // An instance's generated entities are made again by its prefab.
        let alive = (self.scene.entities().iter())
            .filter(|e| self.world.is_alive(e.entity) && e.generated_by.is_none());
        doc.insert(
            ENTITIES,
            Value::List(alive.map(|e| self.entity(e)).collect()),
        );
        Value::Map(doc)
    }

    fn entity(&self, e: &SceneEntity) -> Value {
        let base = self.bases.get(&e.key);
        let mut m = Map::new();
        m.insert(ID, e.key.write());
        if let Some(name) = e
            .name
            .as_ref()
            .filter(|n| Some(*n) != base.and_then(|b| b.name.as_ref()))
        {
            m.insert(NAME, name.write());
        }
        if let Some(prefab) = &e.prefab {
            m.insert(PREFAB, Id::<PrefabKind>::new(prefab.clone()).write());
        }
        if let Some(parent) = self.parent(e) {
            m.insert(PARENT, Id::<EntityKind>::new(parent).write());
        }
        match base {
            Some(base) => self.insert_overrides(&mut m, e, base),
            None => {
                m.insert(COMPONENTS, self.components(e));
            }
        }
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
        for (name, value) in ComponentDocs::of_world(self.registry, self.world, e).iter() {
            m.insert(name.as_str(), value.clone());
        }
        Value::Map(m)
    }

    /// `overrides` of instance `e`: what changed in it and in each entity
    /// of the scene it made, against `base`. Left out when nothing did.
    fn insert_overrides(&self, m: &mut Map, e: &SceneEntity, base: &InstanceOrigin) {
        let mut overrides = Map::new();
        let made = self
            .scene
            .entities()
            .iter()
            .filter(|g| self.world.is_alive(g.entity));
        for member in made.filter(|g| self.instance_of(g) == &e.key) {
            let Some(was) = base.components(&member.key) else {
                continue;
            };
            let now = ComponentDocs::of_world(self.registry, self.world, member);
            if let Some(changes) = now.changes_from(was).into_value() {
                overrides.insert(member.key.as_str(), changes);
            }
        }
        if !overrides.is_empty() {
            m.insert(OVERRIDES, Value::Map(overrides));
        }
    }

    /// The key of the entity in the document that `e` comes from: itself,
    /// or the outermost instance that made it.
    fn instance_of<'s>(&'s self, e: &'s SceneEntity) -> &'s EntityKey {
        let mut at = e;
        for _ in 0..self.scene.entities().len() {
            match at.generated_by.as_ref().and_then(|g| self.scene.get(g)) {
                Some(outer) => at = outer,
                None => break,
            }
        }
        &at.key
    }
}

//! [`DocumentReader`]: reads an `.mscene` document against a registry.

use std::collections::HashMap;

use substrate_data::schema::is_metadata_key;
use substrate_data::{Expected, Id, MapRef, Path, Reader, Resolve, Seg, ValueRef};

use super::DocumentRole;
use super::document_resolver::DocumentResolver;
use super::finding::Finding;
use super::reading::Reading;
use crate::document_keys::{COMPONENTS, ENTITIES, ENTITY, FORMAT_KEY, ID, NAME, PARENT, PREFAB};
use crate::draft::{ComponentsDraft, DocumentDraft, PendingEntity};
use crate::load::SceneProblem;
use crate::{
    ComponentName, ComponentRegistry, DisplayName, EntityKey, EntityKind, FORMAT, KeptComponent,
    PrefabKind, UnknownComponents,
};

/// Reads an `.mscene` document against a registry, collecting every
/// problem Substrate Data doesn't see (unknown components, duplicate keys)
/// next to those it does.
pub(crate) struct DocumentReader<'r> {
    registry: &'r ComponentRegistry,
    role: &'r DocumentRole,
    findings: Vec<Finding>,
    first_at: HashMap<EntityKey, usize>,
}

impl<'r> DocumentReader<'r> {
    /// Reads the document `root`: one draft entity per document entity,
    /// with its registered components. `@entity:` references resolve within
    /// the document, every other kind through `resolver`. Nothing is
    /// spawned: the draft is all or nothing.
    pub(crate) fn read(
        (registry, role): (&'r ComponentRegistry, &'r DocumentRole),
        root: &ValueRef<'_>,
        resolver: &mut dyn Resolve,
    ) -> Reading {
        let mut resolver = DocumentResolver::new(root, resolver, role);
        let mut cx = Reader::with_resolver(&mut resolver);
        let mut reader = DocumentReader {
            registry,
            role,
            findings: Vec::new(),
            first_at: HashMap::new(),
        };
        let draft = reader.read_document(root, &mut cx);
        let problems = std::mem::take(&mut cx.problems);
        let clean = problems.is_empty() && reader.findings.is_empty();
        Reading {
            draft: draft.filter(|_| clean),
            problems,
            findings: reader.findings,
        }
    }

    fn find(&mut self, problem: SceneProblem, path: Path, on_key: bool) {
        self.findings.push(Finding {
            problem,
            path,
            on_key,
        });
    }

    fn read_document(&mut self, root: &ValueRef<'_>, cx: &mut Reader<'_>) -> Option<DocumentDraft> {
        let map = cx.map(root)?;
        cx.unknown_keys(map, self.role.keys());
        let format: Option<u32> = cx.field(map, FORMAT_KEY);
        if let Some(found) = format.filter(|f| *f != FORMAT) {
            self.find(
                SceneProblem::UnsupportedFormat { found },
                cx.path().key(FORMAT_KEY),
                false,
            );
        }
        let name: Option<Option<DisplayName>> = cx.field(map, NAME);
        let entities = self.read_entities(map, cx);
        if self.role.is_prefab() && entities.as_ref().is_some_and(Vec::is_empty) {
            self.find(SceneProblem::PrefabEmpty, cx.path().key(ENTITIES), false);
        }
        Some(DocumentDraft {
            name: name?,
            entities: entities?,
        })
    }

    fn read_entities(
        &mut self,
        map: MapRef<'_>,
        cx: &mut Reader<'_>,
    ) -> Option<Vec<PendingEntity>> {
        let Some(list) = map.get(ENTITIES) else {
            return Some(Vec::new());
        };
        cx.at(Seg::Key(ENTITIES.into()), |cx| {
            let Some(items) = list.as_list() else {
                cx.expected(Expected::List, list);
                return None;
            };
            let read: Vec<Option<PendingEntity>> = (items.iter().enumerate())
                .map(|(i, item)| cx.at(Seg::Index(i), |cx| self.read_entity(i, item, cx)))
                .collect();
            read.into_iter().collect()
        })
    }

    fn read_entity(
        &mut self,
        index: usize,
        v: &ValueRef<'_>,
        cx: &mut Reader<'_>,
    ) -> Option<PendingEntity> {
        let map = cx.map(v)?;
        cx.unknown_keys(map, &ENTITY);
        let key = self.read_key(index, map, cx);
        let name: Option<Option<DisplayName>> = cx.field(map, NAME);
        let prefab: Option<Option<Id<PrefabKind>>> = cx.field(map, PREFAB);
        let parent: Option<Option<Id<EntityKind>>> = cx.field(map, PARENT);
        let components = self.read_components(map, cx);
        let (key, parent) = (key?, parent?.map(|p| p.id));
        if parent.as_ref() == Some(&key) {
            self.find(
                SceneProblem::OwnParent { key },
                cx.path().key(PARENT),
                false,
            );
            return None;
        }
        if !self.check_tree(index, (&key, parent.is_some()), cx) {
            return None;
        }
        let prefab = prefab?.map(|p| p.id);
        Some(PendingEntity {
            index,
            generated_by: None,
            key,
            name: name?,
            prefab,
            parent,
            components: components?,
        })
    }

    /// Whether entity `index` (`key`, with a parent or not) fits a prefab's
    /// tree: the first is the root and has no parent, every other one has.
    /// (A scene's entities may all be roots.)
    fn check_tree(
        &mut self,
        index: usize,
        (key, has_parent): (&EntityKey, bool),
        cx: &Reader<'_>,
    ) -> bool {
        if !self.role.is_prefab() || (index == 0) != has_parent {
            return true;
        }
        let (problem, path) = if index == 0 {
            (
                SceneProblem::PrefabRootHasParent { key: key.clone() },
                cx.path().key(PARENT),
            )
        } else {
            (SceneProblem::PrefabOrphan { key: key.clone() }, cx.path())
        };
        self.find(problem, path, false);
        false
    }

    /// The entity's key, refused when an earlier entity has it.
    fn read_key(
        &mut self,
        index: usize,
        map: MapRef<'_>,
        cx: &mut Reader<'_>,
    ) -> Option<EntityKey> {
        let key: EntityKey = cx.field(map, ID)?;
        if let Some(&first) = self.first_at.get(&key) {
            self.find(
                SceneProblem::DuplicateEntity { key, first },
                cx.path().key(ID),
                false,
            );
            return None;
        }
        self.first_at.insert(key.clone(), index);
        Some(self.role.key_of(key, index == 0))
    }

    fn read_components(
        &mut self,
        entity: MapRef<'_>,
        cx: &mut Reader<'_>,
    ) -> Option<ComponentsDraft> {
        let Some(v) = entity.get(COMPONENTS) else {
            return Some(ComponentsDraft::default());
        };
        cx.at(Seg::Key(COMPONENTS.into()), |cx| {
            let map = cx.map(v)?;
            let mut draft = ComponentsDraft::default();
            let mut ok = true;
            for (key, value) in map.iter().filter(|(k, _)| !is_metadata_key(k)) {
                ok &= self.read_component(key, value, cx, &mut draft);
            }
            ok.then_some(draft)
        })
    }

    /// Component `key` of an entity into `draft`; whether it read.
    fn read_component(
        &mut self,
        key: &str,
        value: &ValueRef<'_>,
        cx: &mut Reader<'_>,
        draft: &mut ComponentsDraft,
    ) -> bool {
        let path = cx.path().key(key);
        let name = match ComponentName::new(key) {
            Ok(name) => name,
            Err(e) => {
                self.find(SceneProblem::BadComponentName(e), path, true);
                return false;
            }
        };
        if let Some(entry) = self.registry.entry(&name) {
            let read = cx.at(Seg::Key(key.into()), |cx| (entry.read)(value, cx));
            return read
                .map(|insert| draft.inserts.push((name, insert)))
                .is_some();
        }
        if self.registry.unknown_components() == UnknownComponents::Keep {
            draft.kept.push(KeptComponent {
                name,
                value: value.to_value(),
            });
            return true;
        }
        let suggestion = name.closest(self.registry.names()).cloned();
        self.find(
            SceneProblem::UnknownComponent { name, suggestion },
            path,
            true,
        );
        false
    }
}

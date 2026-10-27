//! [`DocumentReader`]: reads an `.mscene` document against a registry.

use std::collections::HashMap;

use meridian_ecs::World;
use substrate_data::schema::is_metadata_key;
use substrate_data::{
    Bump, Diagnostic, Doc, Expected, Id, MapRef, Path, Reader, Resolve, Seg, ValueRef, text,
};

use super::document_draft::DocumentDraft;
use super::document_resolver::DocumentResolver;
use super::pending_entity::{ComponentsDraft, PendingEntity};
use super::{LoadError, SceneIssue, SceneProblem};
use crate::document_keys::{
    COMPONENTS, DOCUMENT, ENTITIES, ENTITY, FORMAT_KEY, ID, NAME, PARENT, PREFAB,
};
use crate::{
    ComponentName, ComponentRegistry, DisplayName, EntityKey, EntityKind, FORMAT, KeptComponent,
    PrefabKind, SceneMap, UnknownComponents,
};

/// Reads an `.mscene` document against a registry, collecting every
/// problem Substrate Data doesn't see (unknown components, duplicate keys)
/// next to those it does.
pub(crate) struct DocumentReader<'r> {
    registry: &'r ComponentRegistry,
    findings: Vec<Finding>,
    first_at: HashMap<EntityKey, usize>,
}

/// A problem found here, where it is, and whether at its key.
struct Finding {
    problem: SceneProblem,
    path: Path,
    on_key: bool,
}

impl<'r> DocumentReader<'r> {
    /// Reads `text` into `world`: one entity per document entity, with its
    /// registered components. `@entity:` references resolve within the
    /// document, every other kind through `resolver`. All or nothing.
    pub(crate) fn load(
        registry: &'r ComponentRegistry,
        text: &str,
        world: &mut World,
        resolver: &mut dyn Resolve,
    ) -> Result<SceneMap, LoadError> {
        let doc = text::parse(text).map_err(LoadError::syntax)?;
        let arena = Bump::new();
        let root = doc.value.borrow(&arena);
        let mut resolver = DocumentResolver::new(&root, resolver);
        let mut cx = Reader::with_resolver(&mut resolver);
        let mut reader = DocumentReader {
            registry,
            findings: Vec::new(),
            first_at: HashMap::new(),
        };
        let draft = reader.read_document(&root, &mut cx);
        let problems = std::mem::take(&mut cx.problems);
        match draft {
            Some(draft) if problems.is_empty() && reader.findings.is_empty() => {
                Ok(draft.spawn(world))
            }
            _ => Err(reader.error(problems, (text, &doc))),
        }
    }

    /// Every problem, placed.
    fn error(self, problems: Vec<Diagnostic>, source: (&str, &Doc)) -> LoadError {
        let data = problems
            .into_iter()
            .map(|d| substrate_data::placed(source.0, source.1, d));
        let mut issues: Vec<SceneIssue> = data.map(SceneIssue::of_diagnostic).collect();
        let found = self.findings.into_iter();
        issues.extend(found.map(|f| SceneIssue::placed(f.problem, f.path, f.on_key, source)));
        LoadError::new(issues)
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
        cx.unknown_keys(map, &DOCUMENT);
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
        let prefab = prefab?.map(|p| p.id);
        Some(PendingEntity {
            key,
            name: name?,
            prefab,
            parent,
            components: components?,
        })
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
        Some(key)
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
            return read.map(|insert| draft.inserts.push(insert)).is_some();
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

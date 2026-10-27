//! The `.mscene` document: reading it into a `World`, writing one out.
//!
//! ```jsonc
//! { format: 1, name: "Level 1", entities: [ { id, name?, prefab?, parent?, components: { <name>: <value> } } ] }
//! ```

use std::collections::{HashMap, HashSet};

use meridian_ecs_core::{Entity, World};
use substrate_data::with::{Opt, RefTo};
use substrate_data::{
    Diagnostic, Errors, Map, MapRef, Path, Reader, Ref, Resolve, Seg, Value, ValueRef, placed, text,
};

use crate::registry::{ComponentRegistry, Insert};
use crate::{EntityKind, Parent, PrefabKind};

/// The `.mscene` format version this crate reads and writes. Formats evolve
/// by appending; a bump comes with a `Value -> Value` migration (ADR 016).
pub const FORMAT: i64 = 1;

const KEY_FORMAT: &str = "format";
const KEY_NAME: &str = "name";
const KEY_ENTITIES: &str = "entities";
const KEY_ID: &str = "id";
const KEY_PREFAB: &str = "prefab";
const KEY_PARENT: &str = "parent";
const KEY_COMPONENTS: &str = "components";

/// One entity of a loaded (or to-be-saved) scene: its document id, its
/// runtime handle, and what the document says about it that isn't a
/// component.
#[derive(Debug, Clone, PartialEq)]
pub struct SceneEntity {
    /// Stable within the document; runtime `Entity` handles are never written.
    pub id: String,
    pub entity: Entity,
    pub name: Option<String>,
    /// The prefab it starts from (the reference's id, as the resolver kept it).
    pub prefab: Option<String>,
    /// The `id` of its parent in the same document. On load the parent is
    /// also inserted as a [`Parent`] component; on save a [`Parent`]
    /// component, when the entity has one, wins over this.
    pub parent: Option<String>,
    /// Components the registry doesn't know, kept as written when it
    /// [allows unknown ones](ComponentRegistry::allow_unknown), so saving
    /// writes them back untouched.
    pub unknown: Vec<(String, Value)>,
}

/// A scene's ids and their entities, in document order: what
/// [`ComponentRegistry::load_scene`] returns and
/// [`ComponentRegistry::save_scene`] writes.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct SceneMap {
    pub name: Option<String>,
    pub entities: Vec<SceneEntity>,
}

impl SceneMap {
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds `entity` under document id `id`, for saving a world built in code.
    pub fn push(&mut self, id: impl Into<String>, entity: Entity) -> &mut SceneEntity {
        self.entities.push(SceneEntity {
            id: id.into(),
            entity,
            name: None,
            prefab: None,
            parent: None,
            unknown: Vec::new(),
        });
        self.entities.last_mut().expect("just pushed")
    }

    /// The entity with document id `id`.
    pub fn entity(&self, id: &str) -> Option<Entity> {
        self.get(id).map(|e| e.entity)
    }

    /// The scene entry with document id `id`.
    pub fn get(&self, id: &str) -> Option<&SceneEntity> {
        self.entities.iter().find(|e| e.id == id)
    }

    /// The document id of `entity`.
    pub fn id_of(&self, entity: Entity) -> Option<&str> {
        self.entities
            .iter()
            .find(|e| e.entity == entity)
            .map(|e| e.id.as_str())
    }

    /// The parent the document gives `entity` (the [`Parent`] component
    /// `load_scene` inserted holds the same).
    pub fn parent_of(&self, entity: Entity) -> Option<Entity> {
        let e = self.entities.iter().find(|e| e.entity == entity)?;
        self.entity(e.parent.as_deref()?)
    }
}

/// `@entity:` references checked against the document's own ids; every
/// other kind goes to the caller's resolver.
struct DocumentResolver<'a> {
    ids: HashSet<String>,
    inner: &'a mut dyn Resolve,
}

impl Resolve for DocumentResolver<'_> {
    fn reference(&mut self, r: &Ref, path: &Path) -> Result<String, String> {
        use substrate_data::Kind;
        if r.kind == EntityKind::NAME {
            return if self.ids.contains(&r.id) {
                Ok(r.id.clone())
            } else {
                Err(format!("no entity `{}` in this scene", r.id))
            };
        }
        self.inner.reference(r, path)
    }
}

/// An entity read from the document, not spawned yet.
struct Pending {
    id: String,
    name: Option<String>,
    prefab: Option<String>,
    parent: Option<String>,
    inserts: Vec<Insert>,
    unknown: Vec<(String, Value)>,
}

/// The document's entity ids, read loosely (before checking anything), so
/// `@entity:` references can be resolved wherever they appear.
fn document_ids(root: &ValueRef<'_>) -> HashSet<String> {
    root.get(KEY_ENTITIES)
        .and_then(|e| e.as_list())
        .into_iter()
        .flatten()
        .filter_map(|e| e.get(KEY_ID)?.as_str().map(str::to_owned))
        .collect()
}

impl ComponentRegistry {
    /// Reads `.mscene` `text` into `world`: one entity per document entity,
    /// with its registered components. `resolver` checks the references
    /// the document makes to things outside it (`@asset:`, `@prefab:`);
    /// `@entity:` references are resolved against the document itself.
    ///
    /// All or nothing: on any problem, nothing is spawned and every problem
    /// is returned, in file order, each with its line, column and path.
    pub fn load_scene(
        &self,
        text: &str,
        world: &mut World,
        resolver: &mut dyn Resolve,
    ) -> Result<SceneMap, Errors> {
        let doc = text::parse(text)?;
        let arena = substrate_data::Bump::new();
        let root = doc.value.borrow(&arena);
        let mut resolver = DocumentResolver {
            ids: document_ids(&root),
            inner: resolver,
        };
        let mut cx = Reader::with_resolver(&mut resolver);
        let read = self.read_document(&root, &mut cx);
        let problems = std::mem::take(&mut cx.problems);
        let (name, pending) = match read {
            Some(read) if problems.is_empty() => read,
            _ => {
                let mut problems: Vec<Diagnostic> = problems
                    .into_iter()
                    .map(|d| placed(text, &doc, d))
                    .collect();
                if problems.is_empty() {
                    problems.push(Diagnostic::error("SD0000", "the scene could not be read"));
                }
                problems.sort_by_key(|d| (d.line, d.col));
                return Err(Errors(problems));
            }
        };

        let mut scene = SceneMap {
            name,
            entities: Vec::with_capacity(pending.len()),
        };
        for p in pending {
            let entity = world.spawn();
            for insert in p.inserts {
                insert(world, entity);
            }
            scene.entities.push(SceneEntity {
                id: p.id,
                entity,
                name: p.name,
                prefab: p.prefab,
                parent: p.parent,
                unknown: p.unknown,
            });
        }
        // Parents last: every id has its entity by now (`@entity:` refs
        // were checked against the document while reading).
        for e in &scene.entities {
            if let Some(parent) = e.parent.as_deref().and_then(|p| scene.entity(p)) {
                world.insert(e.entity, Parent(parent));
            }
        }
        Ok(scene)
    }

    fn read_document(
        &self,
        root: &ValueRef<'_>,
        cx: &mut Reader,
    ) -> Option<(Option<String>, Vec<Pending>)> {
        let map = cx.map(root)?;
        cx.unknown_keys(map, &[KEY_FORMAT, KEY_NAME, KEY_ENTITIES]);
        let format: Option<i64> = cx.field(map, KEY_FORMAT);
        if let Some(f) = format
            && f != FORMAT
        {
            cx.at(Seg::Key(KEY_FORMAT.into()), |cx| {
                cx.problem(format!(
                    "unsupported scene format {f} (this reads {FORMAT})"
                ))
            });
        }
        let name: Option<Option<String>> = cx.field(map, KEY_NAME);
        let entities = self.read_entities(map, cx);
        Some((name?, entities?))
    }

    fn read_entities(&self, map: MapRef<'_>, cx: &mut Reader) -> Option<Vec<Pending>> {
        let Some(list) = map.get(KEY_ENTITIES) else {
            return Some(Vec::new());
        };
        cx.at(Seg::Key(KEY_ENTITIES.into()), |cx| {
            let Some(items) = list.as_list() else {
                cx.expected("a list of entities", list);
                return None;
            };
            let mut first_at: HashMap<String, usize> = HashMap::new();
            let mut out = Vec::with_capacity(items.len());
            let mut ok = true;
            for (i, item) in items.iter().enumerate() {
                let read = cx.at(Seg::Index(i), |cx| {
                    self.read_entity(item, i, &mut first_at, cx)
                });
                match read {
                    Some(p) => out.push(p),
                    None => ok = false,
                }
            }
            ok.then_some(out)
        })
    }

    /// Entity `i` of the list; `first_at` maps the ids met so far to their index.
    fn read_entity(
        &self,
        v: &ValueRef<'_>,
        i: usize,
        first_at: &mut HashMap<String, usize>,
        cx: &mut Reader,
    ) -> Option<Pending> {
        let map = cx.map(v)?;
        cx.unknown_keys(
            map,
            &[KEY_ID, KEY_NAME, KEY_PREFAB, KEY_PARENT, KEY_COMPONENTS],
        );
        let mut id: Option<String> = cx.field(map, KEY_ID);
        if let Some(dup) = &id {
            if let Some(first) = first_at.get(dup) {
                cx.at(Seg::Key(KEY_ID.into()), |cx| {
                    cx.problem(format!(
                        "duplicate entity id `{dup}` (first at entities[{first}])"
                    ))
                });
                id = None;
            } else {
                first_at.insert(dup.clone(), i);
            }
        }
        let name: Option<Option<String>> = cx.field(map, KEY_NAME);
        let prefab = cx.field_with::<Opt<RefTo<PrefabKind>>, Option<String>>(map, KEY_PREFAB);
        let parent = cx.field_with::<Opt<RefTo<EntityKind>>, Option<String>>(map, KEY_PARENT);
        if let (Some(id), Some(Some(parent))) = (&id, &parent)
            && id == parent
        {
            cx.at(Seg::Key(KEY_PARENT.into()), |cx| {
                cx.problem(format!("entity `{id}` can't be its own parent"))
            });
            return None;
        }
        let components = self.read_components(map, cx);
        let (inserts, unknown) = components?;
        Some(Pending {
            id: id?,
            name: name?,
            prefab: prefab?,
            parent: parent?,
            inserts,
            unknown,
        })
    }

    #[allow(clippy::type_complexity)]
    fn read_components(
        &self,
        entity: MapRef<'_>,
        cx: &mut Reader,
    ) -> Option<(Vec<Insert>, Vec<(String, Value)>)> {
        let Some(v) = entity.get(KEY_COMPONENTS) else {
            return Some((Vec::new(), Vec::new()));
        };
        cx.at(Seg::Key(KEY_COMPONENTS.into()), |cx| {
            let map = cx.map(v)?;
            let known: Vec<&str> = self.names().collect();
            let (mut inserts, mut unknown, mut ok) = (Vec::new(), Vec::new(), true);
            for (key, value) in map.iter() {
                if substrate_data::schema::is_metadata_key(key) {
                    continue;
                }
                let Some(entry) = self.entry(key) else {
                    if self.allow_unknown {
                        unknown.push((key.to_owned(), value.to_value()));
                    } else {
                        unknown_component(cx, key, &known);
                        ok = false;
                    }
                    continue;
                };
                match cx.at(Seg::Key(key.to_owned()), |cx| (entry.read)(value, cx)) {
                    Some(insert) => inserts.push(insert),
                    None => ok = false,
                }
            }
            ok.then_some((inserts, unknown))
        })
    }

    /// `world`'s entities listed in `scene`, as `.mscene` text: every
    /// registered component each one has, plus its kept unknown ones.
    /// Entities no longer alive are left out. Floats are written so they
    /// read back bit-exactly.
    pub fn save_scene(&self, world: &World, scene: &SceneMap) -> String {
        let mut doc = Map::new();
        doc.insert(KEY_FORMAT, Value::Int(FORMAT));
        if let Some(name) = &scene.name {
            doc.insert(KEY_NAME, plain(name));
        }
        let entities = scene
            .entities
            .iter()
            .filter(|e| world.is_alive(e.entity))
            .map(|e| self.entity_value(world, scene, e))
            .collect();
        doc.insert(KEY_ENTITIES, Value::List(entities));
        text::to_string_pretty(&Value::Map(doc))
    }

    fn entity_value(&self, world: &World, scene: &SceneMap, e: &SceneEntity) -> Value {
        use substrate_data::Kind;
        let mut m = Map::new();
        m.insert(KEY_ID, plain(&e.id));
        if let Some(name) = &e.name {
            m.insert(KEY_NAME, plain(name));
        }
        if let Some(prefab) = &e.prefab {
            m.insert(
                KEY_PREFAB,
                Value::Ref(Ref::new(PrefabKind::NAME, prefab.clone())),
            );
        }
        let parent = match world.get::<Parent>(e.entity) {
            Some(Parent(p)) => scene.id_of(*p).map(str::to_owned),
            None => e.parent.clone(),
        };
        if let Some(parent) = &parent {
            m.insert(
                KEY_PARENT,
                Value::Ref(Ref::new(EntityKind::NAME, parent.clone())),
            );
        }
        let mut components = Map::new();
        for entry in &self.entries {
            if let Some(v) = (entry.write)(world, e.entity) {
                components.insert(entry.name.as_str(), v);
            }
        }
        for (name, v) in &e.unknown {
            if components.get(name).is_none() {
                components.insert(name.as_str(), v.clone());
            }
        }
        m.insert(KEY_COMPONENTS, Value::Map(components));
        Value::Map(m)
    }
}

/// Plain text as the model holds it (a leading `@` doubled).
fn plain(s: &str) -> Value {
    substrate_data::Schema::write(&s.to_owned())
}

/// "unknown component `x` — did you mean `y`?", pointing at the key.
fn unknown_component(cx: &mut Reader, key: &str, known: &[&str]) {
    if known.is_empty() {
        cx.at(Seg::Key(key.to_owned()), |cx| {
            cx.problem(format!(
                "unknown component `{key}` (no components are registered)"
            ))
        });
    } else {
        cx.at(Seg::Key(key.to_owned()), |cx| cx.unknown_name(key, known));
    }
    if let Some(d) = cx.problems.last_mut() {
        d.on_key = true;
        if !d.message.starts_with("unknown component") {
            d.message = d.message.replacen("unknown", "unknown component", 1);
        }
    }
}

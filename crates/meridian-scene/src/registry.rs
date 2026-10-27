//! [`ComponentRegistry`]: component name → shape, read into a `World`,
//! write out of one.

use std::any::TypeId;

use meridian_ecs_core::{Component, Entity, World};
use substrate_data::with::{Plain, With};
use substrate_data::{Map, Reader, Schema, Shape, Value, ValueRef};

/// A read component, waiting for its entity (loading touches the `World`
/// only once the whole document has read without a problem).
pub(crate) type Insert = Box<dyn FnOnce(&mut World, Entity)>;

pub(crate) struct Entry {
    pub(crate) name: String,
    type_id: TypeId,
    shape: fn() -> Shape,
    pub(crate) read: fn(&ValueRef<'_>, &mut Reader) -> Option<Insert>,
    pub(crate) write: fn(&World, Entity) -> Option<Value>,
}

fn read_component<T: Component, W: With<T>>(v: &ValueRef<'_>, cx: &mut Reader) -> Option<Insert> {
    let component = W::read(v, cx)?;
    Some(Box::new(move |world: &mut World, entity| {
        world.insert(entity, component);
    }))
}

fn write_component<T: Component, W: With<T>>(world: &World, entity: Entity) -> Option<Value> {
    world.get::<T>(entity).map(W::write)
}

/// The components a scene may hold, by the name documents use for them.
///
/// Built by the application — no global registry exists (ADR 003): a game
/// registers its own components next to the engine's, an editor builds one
/// to read any scene of that game. One name per component type and one
/// type per name, matching the ECS (one component of a type per entity).
#[derive(Default)]
pub struct ComponentRegistry {
    pub(crate) entries: Vec<Entry>,
    pub(crate) allow_unknown: bool,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers `T` under `name`, read and written by its own [`Schema`].
    /// (`Clone` is required so a prefab can hand each instance its own copy.)
    ///
    /// # Panics
    /// If `name` or `T` is already registered — a programming error in
    /// how the application builds its registry, not a data problem.
    pub fn register<T: Component + Schema + Clone>(&mut self, name: &str) -> &mut Self {
        self.register_with::<T, Plain>(name)
    }

    /// Registers `T` under `name`, read and written through codec `W` — for
    /// a component whose crate doesn't implement [`Schema`] (the orphan
    /// rule keeps it from being implemented anywhere else), such as
    /// `ecs-core`'s `Transform` ([`crate::TransformCodec`]).
    ///
    /// # Panics
    /// As [`ComponentRegistry::register`].
    pub fn register_with<T: Component, W: With<T>>(&mut self, name: &str) -> &mut Self {
        let type_id = TypeId::of::<T>();
        if let Some(e) = self
            .entries
            .iter()
            .find(|e| e.name == name || e.type_id == type_id)
        {
            panic!(
                "component `{name}` ({}): already registered as `{}`",
                std::any::type_name::<T>(),
                e.name
            );
        }
        self.entries.push(Entry {
            name: name.to_owned(),
            type_id,
            shape: W::shape,
            read: read_component::<T, W>,
            write: write_component::<T, W>,
        });
        self
    }

    /// Whether an unknown component name is kept as it is (in
    /// [`crate::SceneEntity::unknown`], written back on save) instead of
    /// being an error. Editors allow it, so a scene written by a newer game
    /// still opens; a game loading its own content doesn't. Default: no.
    pub fn allow_unknown(&mut self, allow: bool) -> &mut Self {
        self.allow_unknown = allow;
        self
    }

    /// The registered names, in registration order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.entries.iter().map(|e| e.name.as_str())
    }

    pub(crate) fn entry(&self, name: &str) -> Option<&Entry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// Every registered component's shape (`Shape::describe()`), by name in
    /// registration order — what an IDE builds its inspector and completion
    /// from.
    pub fn describe(&self) -> Value {
        let mut m = Map::new();
        for e in &self.entries {
            m.insert(e.name.as_str(), (e.shape)().describe());
        }
        Value::Map(m)
    }
}

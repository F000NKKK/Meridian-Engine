//! [`ComponentRegistry`]: the components a scene may hold.

use std::any::TypeId;
use std::collections::HashMap;
use std::fmt;

use meridian_ecs::Component;
use substrate_data::Schema;
use substrate_data::with::{Plain, With};

use super::{ComponentShape, Entry, RegisterError, UnknownComponents};
use crate::ComponentName;

/// The components a scene may hold, by name and by type.
///
/// Built by the application — there is no global registry (ADR 003): a
/// game registers its components next to the engine's, an editor builds
/// one to read that game's scenes. One name per type, one type per name,
/// matching the ECS (one component of a type per entity).
#[derive(Default)]
pub struct ComponentRegistry {
    entries: Vec<Entry>,
    by_name: HashMap<ComponentName, usize>,
    by_type: HashMap<TypeId, usize>,
    unknown: UnknownComponents,
}

impl ComponentRegistry {
    /// An empty registry that rejects unknown components.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The name `T` is registered under.
    #[must_use]
    pub fn name_of<T: 'static>(&self) -> Option<&ComponentName> {
        self.name_of_type(TypeId::of::<T>())
    }

    /// The name the component type `id` is registered under.
    #[must_use]
    pub fn name_of_type(&self, id: TypeId) -> Option<&ComponentName> {
        self.by_type.get(&id).map(|&i| &self.entries[i].name)
    }

    /// The registered names, in registration order.
    pub fn names(&self) -> impl Iterator<Item = &ComponentName> {
        self.entries.iter().map(|e| &e.name)
    }

    /// What loading does with an unknown component name.
    #[must_use]
    pub fn unknown_components(&self) -> UnknownComponents {
        self.unknown
    }

    /// Every registered component's shape, in registration order.
    #[must_use]
    pub fn describe(&self) -> Vec<ComponentShape> {
        let describe = |e: &Entry| ComponentShape {
            name: e.name.clone(),
            shape: (e.shape)().describe(),
        };
        self.entries.iter().map(describe).collect()
    }

    pub(crate) fn entry(&self, name: &ComponentName) -> Option<&Entry> {
        self.by_name.get(name).map(|&i| &self.entries[i])
    }

    pub(crate) fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// Registers `T` under `name`, read and written by its own [`Schema`].
    /// (`Clone`: a prefab gives each instance its own copy.)
    ///
    /// # Errors
    ///
    /// [`RegisterError`] when `name` or `T` is already registered.
    pub fn register<T: Component + Schema + Clone>(
        &mut self,
        name: ComponentName,
    ) -> Result<&mut Self, RegisterError> {
        self.register_with::<T, Plain>(name)
    }

    /// Registers `T` under `name`, read and written through codec `W` — for
    /// a component whose crate doesn't implement [`Schema`], such as
    /// `ecs`'s `Transform` ([`crate::TransformCodec`]).
    ///
    /// # Errors
    ///
    /// [`RegisterError`] when `name` or `T` is already registered.
    pub fn register_with<T: Component, W: With<T>>(
        &mut self,
        name: ComponentName,
    ) -> Result<&mut Self, RegisterError> {
        let type_id = TypeId::of::<T>();
        if self.by_name.contains_key(&name) {
            return Err(RegisterError::NameTaken { name });
        }
        if let Some(taken) = self.name_of_type(type_id) {
            let registered_as = taken.clone();
            return Err(RegisterError::TypeTaken {
                type_name: std::any::type_name::<T>(),
                registered_as,
            });
        }
        self.by_name.insert(name.clone(), self.entries.len());
        self.by_type.insert(type_id, self.entries.len());
        self.entries.push(Entry::of::<T, W>(name));
        Ok(self)
    }

    /// Sets what loading does with an unknown component name.
    pub fn set_unknown_components(&mut self, unknown: UnknownComponents) -> &mut Self {
        self.unknown = unknown;
        self
    }
}

impl fmt::Debug for ComponentRegistry {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ComponentRegistry")
            .field("names", &self.names().collect::<Vec<_>>())
            .field("unknown", &self.unknown)
            .finish()
    }
}

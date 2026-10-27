//! Reading and writing one named component of one entity: what an editor
//! does, as opposed to loading or saving a whole document.

use meridian_ecs::{Entity, World};
use substrate_data::{Bump, Reader, Value};

use super::{ComponentEditError, ComponentRegistry};
use crate::ComponentName;

impl ComponentRegistry {
    /// Whether a component is registered under `name`.
    #[must_use]
    pub fn contains(&self, name: &ComponentName) -> bool {
        self.entry(name).is_some()
    }

    /// `entity`'s component `name` as a value; `None` when the entity lacks
    /// it (or is not alive).
    ///
    /// # Errors
    ///
    /// [`ComponentEditError::Unregistered`] when `name` is not registered.
    pub fn component_value(
        &self,
        world: &World,
        entity: Entity,
        name: &ComponentName,
    ) -> Result<Option<Value>, ComponentEditError> {
        let entry = self.entry_or_error(name)?;
        Ok((entry.write)(world, entity))
    }

    /// Replaces (or adds) `entity`'s component `name` with `value`, read
    /// with the component's own schema. The world is untouched when it fails.
    ///
    /// # Errors
    ///
    /// [`ComponentEditError`]: `name` is not registered, `entity` is not
    /// alive, or `value` is not of the component's shape (every problem, with
    /// its path).
    pub fn set_component(
        &self,
        world: &mut World,
        entity: Entity,
        name: &ComponentName,
        value: &Value,
    ) -> Result<(), ComponentEditError> {
        let entry = self.entry_or_error(name)?;
        if !world.is_alive(entity) {
            return Err(ComponentEditError::NoSuchEntity);
        }
        let arena = Bump::new();
        let mut cx = Reader::new();
        let insert = (entry.read)(&value.borrow(&arena), &mut cx);
        match insert {
            Some(insert) if cx.problems.is_empty() => {
                insert(world, entity);
                Ok(())
            }
            _ => Err(ComponentEditError::Invalid {
                diagnostics: cx.problems,
            }),
        }
    }

    fn entry_or_error(&self, name: &ComponentName) -> Result<&super::Entry, ComponentEditError> {
        self.entry(name)
            .ok_or_else(|| ComponentEditError::Unregistered { name: name.clone() })
    }
}

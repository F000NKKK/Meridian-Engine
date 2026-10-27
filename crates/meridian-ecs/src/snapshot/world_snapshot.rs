//! A copy of a world's entities and components, to roll back to.

use std::collections::HashMap;

use meridian_memory::ResourcePool;

use crate::{Archetype, Cloners, Entity, EntityLocation, SnapshotError, World};

/// A frozen copy of a [`World`]: every live entity with its generation, every
/// component value, and the archetype layout. [`World::restore`] puts the world
/// back exactly, so an [`Entity`] held across the round trip still names the same
/// entity. Format-free and in memory only; writing one to disk is
/// `meridian-data-substrate`'s job, through its own codecs.
#[derive(Debug)]
pub struct WorldSnapshot {
    entities: ResourcePool<()>,
    locations: HashMap<Entity, EntityLocation>,
    archetypes: Vec<Archetype>,
    archetype_lookup: HashMap<Vec<std::any::TypeId>, usize>,
}

impl WorldSnapshot {
    /// How many entities the snapshot holds.
    #[must_use]
    pub fn entity_count(&self) -> usize {
        self.locations.len()
    }
}

impl World {
    /// Copies this world.
    ///
    /// # Errors
    ///
    /// [`SnapshotError::Unregistered`] if an entity holds a component type
    /// `cloners` does not know; nothing is copied then.
    pub fn snapshot(&self, cloners: &Cloners) -> Result<WorldSnapshot, SnapshotError> {
        let mut unregistered: Vec<_> = self
            .archetypes
            .iter()
            .flat_map(|archetype| archetype.columns.keys())
            .filter(|type_id| !cloners.contains(**type_id))
            .copied()
            .collect();
        if !unregistered.is_empty() {
            unregistered.sort();
            unregistered.dedup();
            return Err(SnapshotError::Unregistered(unregistered));
        }
        Ok(WorldSnapshot {
            entities: self.entities.clone(),
            locations: self.locations.clone(),
            archetypes: self
                .archetypes
                .iter()
                .map(|archetype| archetype.clone_with(cloners))
                .collect(),
            archetype_lookup: self.archetype_lookup.clone(),
        })
    }

    /// Puts the world back to `snapshot`: entities spawned since are gone, ones
    /// despawned since are back, component values are as they were.
    ///
    /// The clock does not run backwards. It advances once, and every column is
    /// stamped changed at the new tick, so a system using [`Changed`](crate::Changed)
    /// sees the whole restored world as changed on its next run instead of
    /// missing the rollback.
    pub fn restore(&mut self, snapshot: WorldSnapshot) {
        self.entities = snapshot.entities;
        self.locations = snapshot.locations;
        self.archetypes = snapshot.archetypes;
        self.archetype_lookup = snapshot.archetype_lookup;
        self.advance_tick();
        for archetype in 0..self.archetypes.len() {
            self.mark_archetype(archetype);
        }
    }
}

impl Archetype {
    /// A copy; every column's type was checked against `cloners` first.
    fn clone_with(&self, cloners: &Cloners) -> Archetype {
        let columns = self
            .columns
            .iter()
            .filter_map(|(type_id, column)| {
                let copy = cloners.clone_column(*type_id, column.as_ref())?;
                Some((*type_id, copy))
            })
            .collect();
        Archetype {
            type_ids: self.type_ids.clone(),
            entities: self.entities.clone(),
            columns,
        }
    }
}

//! The world: entities and their archetype storage.

use std::any::TypeId;
use std::collections::HashMap;

use meridian_memory_core::ResourcePool;

use crate::{
    AnyColumn, Archetype, Column, Component, Entity, EntityLocation, Query, QueryMany,
    QueryManyMut, QueryMut, ReadSelection, Selection,
};

/// Owns entity allocation, every archetype, and their `SoA` storage. Not a
/// global singleton — the application creates and owns a `World` like any
/// other value, consistent with [ADR 003](../../../docs/adr/003-no-global-managers.md).
#[derive(Debug, Default)]
pub struct World {
    entities: ResourcePool<()>,
    locations: HashMap<Entity, EntityLocation>,
    archetypes: Vec<Archetype>,
    archetype_lookup: HashMap<Vec<TypeId>, usize>,
}

impl World {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the index of the archetype for `signature` (a sorted list
    /// of `TypeId`s), creating it — with columns cloned in shape from
    /// `source`'s matching entries via `new_same_type`, plus `extra` if
    /// given — if it doesn't exist yet.
    fn archetype_for(
        &mut self,
        signature: Vec<TypeId>,
        source: Option<usize>,
        extra: Option<(TypeId, Box<dyn AnyColumn>)>,
    ) -> usize {
        if let Some(&idx) = self.archetype_lookup.get(&signature) {
            return idx;
        }
        let mut columns: HashMap<TypeId, Box<dyn AnyColumn>> = HashMap::new();
        if let Some(source) = source {
            for (tid, col) in &self.archetypes[source].columns {
                if signature.contains(tid) {
                    columns.insert(*tid, col.new_same_type());
                }
            }
        }
        if let Some((tid, col)) = extra {
            columns.insert(tid, col);
        }
        let idx = self.archetypes.len();
        self.archetypes.push(Archetype {
            type_ids: signature.clone(),
            entities: Vec::new(),
            columns,
        });
        self.archetype_lookup.insert(signature, idx);
        idx
    }

    pub fn spawn(&mut self) -> Entity {
        let entity: Entity = self.entities.insert(()).into();
        let archetype = self.archetype_for(Vec::new(), None, None);
        let row = self.archetypes[archetype].entities.len();
        self.archetypes[archetype].entities.push(entity);
        self.locations
            .insert(entity, EntityLocation { archetype, row });
        entity
    }

    /// Every live entity, in no particular order.
    pub fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.locations.keys().copied()
    }

    #[must_use]
    pub fn is_alive(&self, entity: Entity) -> bool {
        self.locations.contains_key(&entity)
    }

    /// Removes `row` from `archetype` (swap-remove: the last row takes its
    /// place), fixing up the displaced entity's recorded row if there was
    /// one.
    fn remove_row(&mut self, archetype: usize, row: usize) {
        let arch = &mut self.archetypes[archetype];
        arch.entities.swap_remove(row);
        for col in arch.columns.values_mut() {
            col.swap_remove_erased(row);
        }
        if let Some(&moved_entity) = arch.entities.get(row) {
            self.locations
                .get_mut(&moved_entity)
                .expect("every entity stored in an archetype has a location")
                .row = row;
        }
    }

    pub fn despawn(&mut self, entity: Entity) -> bool {
        let Some(loc) = self.locations.remove(&entity) else {
            return false;
        };
        self.entities.remove(entity.into());
        self.remove_row(loc.archetype, loc.row);
        true
    }

    /// Returns `(&mut archetypes[i], &mut archetypes[j])` regardless of
    /// which index is larger. Panics if `i == j` (a caller bug: moving an
    /// entity to its own archetype should never reach here).
    fn archetypes_mut2(&mut self, i: usize, j: usize) -> (&mut Archetype, &mut Archetype) {
        assert_ne!(
            i, j,
            "archetypes_mut2: cannot borrow the same archetype twice"
        );
        if i < j {
            let (left, right) = self.archetypes.split_at_mut(j);
            (&mut left[i], &mut right[0])
        } else {
            let (left, right) = self.archetypes.split_at_mut(i);
            (&mut right[0], &mut left[j])
        }
    }

    /// Moves `entity`'s row from `old_arch` to `new_arch`, carrying over
    /// every column shared by both (via `AnyColumn::move_row_to`).
    /// `skip_type`, if set, is a column the caller already extracted a
    /// value from by hand (see [`World::remove`]) — the generic loop
    /// leaves it alone rather than double-removing that row.
    fn move_entity(
        &mut self,
        entity: Entity,
        from: EntityLocation,
        new_arch: usize,
        skip_type: Option<TypeId>,
    ) {
        let EntityLocation {
            archetype: old_arch,
            row: old_row,
        } = from;
        let (old, new) = self.archetypes_mut2(old_arch, new_arch);
        old.entities.swap_remove(old_row);
        new.entities.push(entity);
        let new_row = new.entities.len() - 1;

        for (tid, old_col) in &mut old.columns {
            if Some(*tid) == skip_type {
                continue;
            }
            if let Some(new_col) = new.columns.get_mut(tid) {
                old_col.move_row_to(old_row, new_col.as_mut());
            } else {
                old_col.swap_remove_erased(old_row);
            }
        }

        if let Some(&moved_entity) = old.entities.get(old_row) {
            self.locations
                .get_mut(&moved_entity)
                .expect("every entity stored in an archetype has a location")
                .row = old_row;
        }
        self.locations.insert(
            entity,
            EntityLocation {
                archetype: new_arch,
                row: new_row,
            },
        );
    }

    /// Adds `component` to `entity`, moving it into (creating if needed)
    /// the archetype for its new component set. If `entity` already has a
    /// `T`, overwrites it in place — no archetype move. Returns `false` if
    /// `entity` isn't alive.
    pub fn insert<T: Component>(&mut self, entity: Entity, component: T) -> bool {
        let Some(&EntityLocation {
            archetype: old_arch,
            row: old_row,
        }) = self.locations.get(&entity)
        else {
            return false;
        };
        let type_id = TypeId::of::<T>();

        if self.archetypes[old_arch].type_ids.contains(&type_id) {
            self.archetypes[old_arch].column_mut::<T>()[old_row] = component;
            return true;
        }

        let mut new_signature = self.archetypes[old_arch].type_ids.clone();
        new_signature.push(type_id);
        new_signature.sort_unstable();

        let extra: Box<dyn AnyColumn> = Box::new(Column::<T>(Vec::new()));
        let new_arch = self.archetype_for(new_signature, Some(old_arch), Some((type_id, extra)));

        self.move_entity(
            entity,
            EntityLocation {
                archetype: old_arch,
                row: old_row,
            },
            new_arch,
            None,
        );

        self.archetypes[new_arch].column_mut::<T>().push(component);
        true
    }

    /// Removes `T` from `entity`, moving it into (creating if needed) the
    /// archetype for its remaining component set. Returns the removed
    /// value, or `None` if `entity` isn't alive or doesn't have a `T`.
    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        let &EntityLocation {
            archetype: old_arch,
            row: old_row,
        } = self.locations.get(&entity)?;
        let type_id = TypeId::of::<T>();
        if !self.archetypes[old_arch].type_ids.contains(&type_id) {
            return None;
        }

        let mut new_signature = self.archetypes[old_arch].type_ids.clone();
        new_signature.retain(|&id| id != type_id);

        let new_arch = self.archetype_for(new_signature, Some(old_arch), None);

        let removed = self.archetypes[old_arch]
            .column_mut::<T>()
            .swap_remove(old_row);

        self.move_entity(
            entity,
            EntityLocation {
                archetype: old_arch,
                row: old_row,
            },
            new_arch,
            Some(type_id),
        );

        Some(removed)
    }

    #[must_use]
    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        let loc = self.locations.get(&entity)?;
        let col = self.archetypes[loc.archetype]
            .columns
            .get(&TypeId::of::<T>())?;
        col.as_any().downcast_ref::<Column<T>>()?.0.get(loc.row)
    }

    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let loc = *self.locations.get(&entity)?;
        let col = self.archetypes[loc.archetype]
            .columns
            .get_mut(&TypeId::of::<T>())?;
        col.as_any_mut()
            .downcast_mut::<Column<T>>()?
            .0
            .get_mut(loc.row)
    }

    /// The component types `entity` currently has, sorted, or `None` for a
    /// dead entity. This is the archetype's identity, so the call allocates
    /// nothing. `TypeId` is not stable across builds: a persistence layer
    /// keeps its own registry from stable names to concrete types and uses
    /// this to know which of them to read with [`World::get`].
    #[must_use]
    pub fn component_types(&self, entity: Entity) -> Option<&[TypeId]> {
        let loc = self.locations.get(&entity)?;
        Some(&self.archetypes[loc.archetype].type_ids)
    }

    #[must_use]
    pub fn contains<T: Component>(&self, entity: Entity) -> bool {
        self.get::<T>(entity).is_some()
    }

    /// Iterates `(Entity, &T)` for every entity that has a `T`, across
    /// every archetype that includes it.
    #[must_use]
    pub fn query<T: Component>(&self) -> Query<'_, T> {
        let type_id = TypeId::of::<T>();
        let inner = self.archetypes.iter().flat_map(move |arch| {
            arch.columns
                .get(&type_id)
                .and_then(|col| col.as_any().downcast_ref::<Column<T>>())
                .into_iter()
                .flat_map(|col| arch.entities.iter().copied().zip(col.0.iter()))
        });
        Query {
            inner: Box::new(inner),
        }
    }

    /// Iterates `(Entity, &mut T)` for every entity that has a `T`, across
    /// every archetype that includes it.
    pub fn query_mut<T: Component>(&mut self) -> QueryMut<'_, T> {
        let type_id = TypeId::of::<T>();
        let inner = self.archetypes.iter_mut().flat_map(move |arch| {
            arch.columns
                .get_mut(&type_id)
                .and_then(|col| col.as_any_mut().downcast_mut::<Column<T>>())
                .into_iter()
                .flat_map(|col| arch.entities.iter().copied().zip(col.0.iter_mut()))
        });
        QueryMut {
            inner: Box::new(inner),
        }
    }

    /// Iterates `(Entity, (&A, &B, ..))` for every entity that has all the
    /// components in the tuple `Q` (two to four `&T` elements).
    #[must_use]
    pub fn query_many<Q: ReadSelection>(&self) -> QueryMany<'_, Q> {
        QueryMany::new(self.archetypes.iter())
    }

    /// Iterates `(Entity, (&A, &mut B, ..))` for every entity that has all
    /// the components in the tuple `Q` (two to four `&T` / `&mut T`
    /// elements).
    ///
    /// # Panics
    ///
    /// If `Q` names one component type twice, since `(&mut T, &T)` would
    /// alias the same column.
    pub fn query_many_mut<Q: Selection>(&mut self) -> QueryManyMut<'_, Q> {
        QueryManyMut::new(self.archetypes.iter_mut())
    }
}

//! Archetype-based, data-oriented Entity Component System with SoA storage, built for cache- and SIMD-friendly iteration.
//!
//! `World` owns everything: entity allocation, archetypes, and per-
//! archetype Structure-of-Arrays component storage. Entities move between
//! archetypes on [`World::insert`]/[`World::remove`] — the SoA columns are
//! type-erased (via `std::any::Any` + downcasting) so `World` never needs
//! to know every `Component` type that will ever exist, but every move is
//! still a safe, checked downcast, no `unsafe`.
//!
//! Scope, deliberately: [`Query`]/[`QueryMut`] iterate a *single*
//! component type across every archetype that has it. Multi-component
//! queries (`Query<(&Transform, &mut Velocity)>`) need to prove to the
//! borrow checker that two different columns don't alias, which is a
//! genuinely harder, `unsafe`-adjacent problem — deferred until there's a
//! system that actually needs it, not built speculatively. See
//! docs/ecs-design.md.

use std::any::{Any, TypeId};
use std::collections::HashMap;

use meridian_gac_core::Motor3;
use meridian_memory_core::{Handle, ResourcePool};

/// An opaque, generational entity id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Entity {
    pub index: u32,
    pub generation: u32,
}

impl From<Handle> for Entity {
    fn from(h: Handle) -> Self {
        Entity { index: h.index, generation: h.generation }
    }
}

impl From<Entity> for Handle {
    fn from(e: Entity) -> Self {
        Handle { index: e.index, generation: e.generation }
    }
}

/// Marker trait for plain-data component types. No behavior — see
/// docs/ecs-design.md.
pub trait Component: 'static {}

/// A type-erased SoA column, downcast back to `Column<T>` by every
/// operation that needs the concrete type. Object-safe by hand (`Any`
/// isn't dyn-compatible for downcasting on its own) — every method here
/// is what a caller holding only `&dyn AnyColumn` can still do safely.
trait AnyColumn {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
    fn len(&self) -> usize;
    /// A fresh, empty column of the same concrete type — how a new
    /// archetype clones the *shape* of an existing one without `World`
    /// ever needing to know the concrete `T`.
    fn new_same_type(&self) -> Box<dyn AnyColumn>;
    /// Removes `row` from this column and pushes it onto `dest` (which
    /// must be the same concrete type — an archetype move never changes a
    /// surviving column's element type). Panics on a type mismatch; that
    /// would be an internal `World` bug, not a caller error.
    fn move_row_to(&mut self, row: usize, dest: &mut dyn AnyColumn);
    /// Drops `row` without moving it anywhere (the component isn't in the
    /// destination archetype's signature).
    fn swap_remove_erased(&mut self, row: usize);
}

struct Column<T>(Vec<T>);

impl<T: Component> AnyColumn for Column<T> {
    fn as_any(&self) -> &dyn Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
    fn len(&self) -> usize {
        self.0.len()
    }
    fn new_same_type(&self) -> Box<dyn AnyColumn> {
        Box::new(Column::<T>(Vec::new()))
    }
    fn move_row_to(&mut self, row: usize, dest: &mut dyn AnyColumn) {
        let value = self.0.swap_remove(row);
        let dest = dest
            .as_any_mut()
            .downcast_mut::<Column<T>>()
            .expect("AnyColumn::move_row_to: destination column type mismatch");
        dest.0.push(value);
    }
    fn swap_remove_erased(&mut self, row: usize) {
        self.0.swap_remove(row);
    }
}

/// The set of component types an entity has; entities sharing an archetype
/// share Structure-of-Arrays storage (each `Column<T>` here is one SoA
/// column, keyed by `TypeId`).
struct ArchetypeData {
    type_ids: Vec<TypeId>, // sorted; the archetype's identity
    entities: Vec<Entity>, // row -> Entity
    columns: HashMap<TypeId, Box<dyn AnyColumn>>,
}

#[derive(Clone, Copy)]
struct EntityLocation {
    archetype: usize,
    row: usize,
}

/// Owns entity allocation, every archetype, and their SoA storage. Not a
/// global singleton — the application creates and owns a `World` like any
/// other value, consistent with [ADR 003](../../../docs/adr/003-no-global-managers.md).
#[derive(Default)]
pub struct World {
    entities: ResourcePool<()>,
    locations: HashMap<Entity, EntityLocation>,
    archetypes: Vec<ArchetypeData>,
    archetype_lookup: HashMap<Vec<TypeId>, usize>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the index of the archetype for `signature` (a sorted list
    /// of `TypeId`s), creating it — with columns cloned in shape from
    /// `source`'s matching entries via `new_same_type`, plus `extra` if
    /// given — if it doesn't exist yet.
    fn get_or_create_archetype(
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
        self.archetypes.push(ArchetypeData { type_ids: signature.clone(), entities: Vec::new(), columns });
        self.archetype_lookup.insert(signature, idx);
        idx
    }

    pub fn spawn(&mut self) -> Entity {
        let entity: Entity = self.entities.insert(()).into();
        let archetype = self.get_or_create_archetype(Vec::new(), None, None);
        let row = self.archetypes[archetype].entities.len();
        self.archetypes[archetype].entities.push(entity);
        self.locations.insert(entity, EntityLocation { archetype, row });
        entity
    }

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
            self.locations.get_mut(&moved_entity).unwrap().row = row;
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
    fn archetypes_mut2(&mut self, i: usize, j: usize) -> (&mut ArchetypeData, &mut ArchetypeData) {
        assert_ne!(i, j, "archetypes_mut2: cannot borrow the same archetype twice");
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
        old_arch: usize,
        old_row: usize,
        new_arch: usize,
        skip_type: Option<TypeId>,
    ) {
        let (old, new) = self.archetypes_mut2(old_arch, new_arch);
        old.entities.swap_remove(old_row);
        new.entities.push(entity);
        let new_row = new.entities.len() - 1;

        for (tid, old_col) in old.columns.iter_mut() {
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
            self.locations.get_mut(&moved_entity).unwrap().row = old_row;
        }
        self.locations.insert(entity, EntityLocation { archetype: new_arch, row: new_row });
    }

    /// Adds `component` to `entity`, moving it into (creating if needed)
    /// the archetype for its new component set. If `entity` already has a
    /// `T`, overwrites it in place — no archetype move. Returns `false` if
    /// `entity` isn't alive.
    pub fn insert<T: Component>(&mut self, entity: Entity, component: T) -> bool {
        let Some(&EntityLocation { archetype: old_arch, row: old_row }) = self.locations.get(&entity) else {
            return false;
        };
        let type_id = TypeId::of::<T>();

        if self.archetypes[old_arch].type_ids.contains(&type_id) {
            let col = self.archetypes[old_arch].columns.get_mut(&type_id).unwrap();
            col.as_any_mut().downcast_mut::<Column<T>>().unwrap().0[old_row] = component;
            return true;
        }

        let mut new_signature = self.archetypes[old_arch].type_ids.clone();
        new_signature.push(type_id);
        new_signature.sort_unstable();

        let extra: Box<dyn AnyColumn> = Box::new(Column::<T>(Vec::new()));
        let new_arch = self.get_or_create_archetype(new_signature, Some(old_arch), Some((type_id, extra)));

        self.move_entity(entity, old_arch, old_row, new_arch, None);

        let col = self.archetypes[new_arch].columns.get_mut(&type_id).unwrap();
        col.as_any_mut().downcast_mut::<Column<T>>().unwrap().0.push(component);
        true
    }

    /// Removes `T` from `entity`, moving it into (creating if needed) the
    /// archetype for its remaining component set. Returns the removed
    /// value, or `None` if `entity` isn't alive or doesn't have a `T`.
    pub fn remove<T: Component>(&mut self, entity: Entity) -> Option<T> {
        let &EntityLocation { archetype: old_arch, row: old_row } = self.locations.get(&entity)?;
        let type_id = TypeId::of::<T>();
        if !self.archetypes[old_arch].type_ids.contains(&type_id) {
            return None;
        }

        let mut new_signature = self.archetypes[old_arch].type_ids.clone();
        new_signature.retain(|&id| id != type_id);

        let new_arch = self.get_or_create_archetype(new_signature, Some(old_arch), None);

        let removed = {
            let col = self.archetypes[old_arch].columns.get_mut(&type_id).unwrap();
            col.as_any_mut().downcast_mut::<Column<T>>().unwrap().0.swap_remove(old_row)
        };

        self.move_entity(entity, old_arch, old_row, new_arch, Some(type_id));

        Some(removed)
    }

    pub fn get<T: Component>(&self, entity: Entity) -> Option<&T> {
        let loc = self.locations.get(&entity)?;
        let col = self.archetypes[loc.archetype].columns.get(&TypeId::of::<T>())?;
        col.as_any().downcast_ref::<Column<T>>()?.0.get(loc.row)
    }

    pub fn get_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let loc = *self.locations.get(&entity)?;
        let col = self.archetypes[loc.archetype].columns.get_mut(&TypeId::of::<T>())?;
        col.as_any_mut().downcast_mut::<Column<T>>()?.0.get_mut(loc.row)
    }

    pub fn contains<T: Component>(&self, entity: Entity) -> bool {
        self.get::<T>(entity).is_some()
    }

    /// Iterates `(Entity, &T)` for every entity that has a `T`, across
    /// every archetype that includes it.
    pub fn query<T: Component>(&self) -> Query<'_, T> {
        let type_id = TypeId::of::<T>();
        let inner = self.archetypes.iter().flat_map(move |arch| {
            arch.columns
                .get(&type_id)
                .and_then(|col| col.as_any().downcast_ref::<Column<T>>())
                .into_iter()
                .flat_map(|col| arch.entities.iter().copied().zip(col.0.iter()))
        });
        Query { inner: Box::new(inner) }
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
        QueryMut { inner: Box::new(inner) }
    }
}

/// Iterates `(Entity, &T)` for every entity with a `T` component. See
/// [`World::query`].
pub struct Query<'w, T> {
    inner: Box<dyn Iterator<Item = (Entity, &'w T)> + 'w>,
}

impl<'w, T> Iterator for Query<'w, T> {
    type Item = (Entity, &'w T);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// Iterates `(Entity, &mut T)` for every entity with a `T` component. See
/// [`World::query_mut`].
pub struct QueryMut<'w, T> {
    inner: Box<dyn Iterator<Item = (Entity, &'w mut T)> + 'w>,
}

impl<'w, T> Iterator for QueryMut<'w, T> {
    type Item = (Entity, &'w mut T);
    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

/// The engine-wide spatial component: rotation + translation as a single
/// motor, shared with physics/graphics/audio (see docs/gac-design.md).
#[derive(Debug, Clone, Copy, Default)]
pub struct Transform {
    pub motor: Motor3,
}

impl Component for Transform {}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Position(f32, f32);
    impl Component for Position {}

    #[derive(Debug, Clone, Copy, PartialEq)]
    struct Velocity(f32, f32);
    impl Component for Velocity {}

    #[test]
    fn spawn_creates_alive_entity() {
        let mut world = World::new();
        let e = world.spawn();
        assert!(world.is_alive(e));
    }

    #[test]
    fn despawn_kills_entity() {
        let mut world = World::new();
        let e = world.spawn();
        assert!(world.despawn(e));
        assert!(!world.is_alive(e));
        assert!(!world.despawn(e), "double despawn must return false, not panic");
    }

    #[test]
    fn insert_and_get_roundtrip() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Position(1.0, 2.0));
        assert_eq!(world.get::<Position>(e), Some(&Position(1.0, 2.0)));
    }

    #[test]
    fn insert_on_existing_component_overwrites_in_place() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Position(1.0, 2.0));
        world.insert(e, Position(9.0, 9.0));
        assert_eq!(world.get::<Position>(e), Some(&Position(9.0, 9.0)));
    }

    #[test]
    fn insert_moves_entity_across_archetypes_preserving_existing_components() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Position(1.0, 2.0));
        world.insert(e, Velocity(3.0, 4.0));

        // The archetype move (Position-only -> Position+Velocity) must not
        // lose or corrupt Position's already-stored value.
        assert_eq!(world.get::<Position>(e), Some(&Position(1.0, 2.0)));
        assert_eq!(world.get::<Velocity>(e), Some(&Velocity(3.0, 4.0)));
    }

    #[test]
    fn remove_takes_value_out_and_entity_survives() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Position(1.0, 2.0));
        world.insert(e, Velocity(3.0, 4.0));

        assert_eq!(world.remove::<Velocity>(e), Some(Velocity(3.0, 4.0)));
        assert!(world.is_alive(e));
        assert_eq!(world.get::<Velocity>(e), None);
        assert_eq!(world.get::<Position>(e), Some(&Position(1.0, 2.0)), "removing Velocity must not disturb Position");
    }

    #[test]
    fn remove_missing_component_returns_none() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Position(1.0, 2.0));
        assert_eq!(world.remove::<Velocity>(e), None);
    }

    #[test]
    fn get_on_dead_or_unknown_entity_returns_none_not_panic() {
        let world = World::new();
        let fake = Entity { index: 999, generation: 0 };
        assert_eq!(world.get::<Position>(fake), None);
    }

    #[test]
    fn despawn_swap_remove_fixes_up_displaced_entity_location() {
        let mut world = World::new();
        let a = world.spawn();
        world.insert(a, Position(1.0, 1.0));
        let b = world.spawn();
        world.insert(b, Position(2.0, 2.0));
        let c = world.spawn();
        world.insert(c, Position(3.0, 3.0));

        // Despawning `a` (row 0) swap-removes the last row (c) into its
        // place; `b` must still resolve correctly afterward.
        world.despawn(a);
        assert_eq!(world.get::<Position>(b), Some(&Position(2.0, 2.0)));
        assert_eq!(world.get::<Position>(c), Some(&Position(3.0, 3.0)));
    }

    #[test]
    fn query_spans_every_archetype_containing_the_component() {
        let mut world = World::new();
        let a = world.spawn();
        world.insert(a, Position(1.0, 0.0));

        let b = world.spawn();
        world.insert(b, Position(2.0, 0.0));
        world.insert(b, Velocity(0.0, 0.0)); // b lives in a different archetype than a

        let mut seen: Vec<(Entity, Position)> = world.query::<Position>().map(|(e, p)| (e, *p)).collect();
        seen.sort_by_key(|(e, _)| e.index);

        assert_eq!(seen, vec![(a, Position(1.0, 0.0)), (b, Position(2.0, 0.0))]);
    }

    #[test]
    fn query_mut_allows_in_place_mutation() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Position(1.0, 1.0));

        for (_, pos) in world.query_mut::<Position>() {
            pos.0 += 10.0;
        }

        assert_eq!(world.get::<Position>(e), Some(&Position(11.0, 1.0)));
    }

    #[test]
    fn transform_is_a_component_and_defaults_to_identity_motor() {
        let mut world = World::new();
        let e = world.spawn();
        world.insert(e, Transform::default());
        assert_eq!(world.get::<Transform>(e).unwrap().motor, Motor3::identity());
    }
}

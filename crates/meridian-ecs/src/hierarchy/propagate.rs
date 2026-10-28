//! Resolves every [`Transform`] into a [`GlobalTransform`] along [`Parent`] links.

use meridian_gac::Motor3;

use super::{GlobalTransform, Parent};
use crate::{Entity, Transform, World};

/// Writes a [`GlobalTransform`] for every entity with a [`Transform`]: the
/// local motor composed with its parent's global motor
/// (`local.compose(parent)`, the order of `gac-wgpu`'s batch kernels).
///
/// An entity is a root, so its global motor is its local one, when it has no
/// [`Parent`], when the parent is despawned or has no [`Transform`], or when
/// it sits on a cycle of parents (every member of a cycle is a root, so the
/// result does not depend on the order entities are visited in).
///
/// A `GlobalTransform` is rewritten only when its value changes, so
/// `Changed<GlobalTransform>` filters see exactly the entities that moved.
/// Returns how many were written. The whole world is resolved on every call;
/// skipping subtrees whose `Transform` did not change is a later refinement.
pub fn propagate_transforms(world: &mut World) -> usize {
    let mut table = Table::of(world);
    let mut chain = Vec::new();
    let entities: Vec<Entity> = table.entities().collect();
    for &entity in &entities {
        resolve(world, &mut table, entity, &mut chain);
    }
    entities
        .into_iter()
        .filter(|&entity| store(world, entity, table.global(entity)))
        .count()
}

/// The entities with a `Transform`, by index: a slot per entity index, so
/// a lookup is an array access and a stale generation reads as absent.
struct Table {
    slots: Vec<Option<Slot>>,
}

struct Slot {
    entity: Entity,
    local: Motor3,
    global: Option<Motor3>,
}

impl Table {
    fn of(world: &World) -> Self {
        let mut slots: Vec<Option<Slot>> = Vec::new();
        for (entity, transform) in world.query::<Transform>() {
            let at = entity.index as usize;
            if slots.len() <= at {
                slots.resize_with(at + 1, || None);
            }
            slots[at] = Some(Slot {
                entity,
                local: transform.motor,
                global: None,
            });
        }
        Self { slots }
    }

    fn entities(&self) -> impl Iterator<Item = Entity> + '_ {
        self.slots.iter().flatten().map(|slot| slot.entity)
    }

    fn slot(&self, entity: Entity) -> Option<&Slot> {
        self.slots
            .get(entity.index as usize)?
            .as_ref()
            .filter(|slot| slot.entity == entity)
    }

    fn slot_mut(&mut self, entity: Entity) -> Option<&mut Slot> {
        self.slots
            .get_mut(entity.index as usize)?
            .as_mut()
            .filter(|slot| slot.entity == entity)
    }

    fn local(&self, entity: Entity) -> Motor3 {
        self.slot(entity)
            .map_or_else(Motor3::identity, |slot| slot.local)
    }

    fn resolved(&self, entity: Entity) -> Option<Motor3> {
        self.slot(entity)?.global
    }

    fn global(&self, entity: Entity) -> Motor3 {
        self.resolved(entity).unwrap_or_else(|| self.local(entity))
    }

    fn set(&mut self, entity: Entity, global: Motor3) {
        if let Some(slot) = self.slot_mut(entity) {
            slot.global = Some(global);
        }
    }
}

/// Resolves `start` and the ancestors it needs. `chain` is scratch space:
/// `start` and the unresolved ancestors above it.
fn resolve(world: &World, table: &mut Table, start: Entity, chain: &mut Vec<Entity>) {
    chain.clear();
    let mut current = start;
    let mut above = loop {
        if let Some(motor) = table.resolved(current) {
            break Some(motor);
        }
        if let Some(at) = chain.iter().position(|&seen| seen == current) {
            for member in chain.split_off(at) {
                table.set(member, table.local(member));
            }
            break table.resolved(current);
        }
        chain.push(current);
        match parent_of(world, table, current) {
            Some(parent) => current = parent,
            None => break None,
        }
    };
    while let Some(entity) = chain.pop() {
        let local = table.local(entity);
        let global = above.map_or(local, |parent| local.compose(parent));
        table.set(entity, global);
        above = Some(global);
    }
}

/// The parent of `entity` if it is an entity with a `Transform`.
fn parent_of(world: &World, table: &Table, entity: Entity) -> Option<Entity> {
    let Parent(parent) = *world.get::<Parent>(entity)?;
    table.slot(parent).map(|slot| slot.entity)
}

/// Stores `motor` as the entity's `GlobalTransform`; whether that changed it.
fn store(world: &mut World, entity: Entity, motor: Motor3) -> bool {
    match world.get::<GlobalTransform>(entity) {
        Some(current) if current.motor == motor => false,
        Some(_) => {
            if let Some(global) = world.get_mut::<GlobalTransform>(entity) {
                global.motor = motor;
            }
            true
        }
        None => world.insert(entity, GlobalTransform { motor }),
    }
}

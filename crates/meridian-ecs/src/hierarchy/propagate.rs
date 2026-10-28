//! Resolves every [`Transform`] into a [`GlobalTransform`] along [`Parent`] links.

use meridian_gac::{Motor3, Scale3};

use super::{GlobalTransform, Parent};
use crate::{Entity, Transform, World};

/// Writes a [`GlobalTransform`] for every entity with a [`Transform`]: the
/// local motor composed with its parent's global motor
/// (`local.compose(parent)`, the order of `gac-wgpu`'s batch kernels), and the
/// entity's [`Scale3`] (the unit one when it has none) multiplied by the
/// parent's global scale. The child's translation stretches with the parent's
/// scale, so a child half a metre to the right of a parent scaled by two sits
/// a metre away (`T * R * S` down the chain). An unscaled chain takes the
/// exact path it always had.
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

/// A transform: the motor and the scale that together place an entity.
#[derive(Clone, Copy)]
struct Placement {
    motor: Motor3,
    scale: Scale3,
}

impl Placement {
    /// `self` placed under `parent`, which is already resolved: the scales
    /// multiply, and the translation stretches with the parent's scale before
    /// the parent's motor places it.
    fn under(self, parent: Placement) -> Placement {
        Placement {
            motor: self.motor.stretched(parent.scale).compose(parent.motor),
            scale: self.scale.compose(parent.scale),
        }
    }
}

struct Slot {
    entity: Entity,
    local: Placement,
    global: Option<Placement>,
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
                local: Placement {
                    motor: transform.motor,
                    scale: world.get::<Scale3>(entity).copied().unwrap_or_default(),
                },
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

    fn local(&self, entity: Entity) -> Placement {
        self.slot(entity).map_or(
            Placement {
                motor: Motor3::identity(),
                scale: Scale3::UNIT,
            },
            |slot| slot.local,
        )
    }

    fn resolved(&self, entity: Entity) -> Option<Placement> {
        self.slot(entity)?.global
    }

    fn global(&self, entity: Entity) -> Placement {
        self.resolved(entity).unwrap_or_else(|| self.local(entity))
    }

    fn set(&mut self, entity: Entity, global: Placement) {
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
        let global = above.map_or(local, |parent| local.under(parent));
        table.set(entity, global);
        above = Some(global);
    }
}

/// The parent of `entity` if it is an entity with a `Transform`.
fn parent_of(world: &World, table: &Table, entity: Entity) -> Option<Entity> {
    let Parent(parent) = *world.get::<Parent>(entity)?;
    table.slot(parent).map(|slot| slot.entity)
}

/// Stores `placement` as the entity's `GlobalTransform`; whether that changed it.
fn store(world: &mut World, entity: Entity, placement: Placement) -> bool {
    let next = GlobalTransform {
        motor: placement.motor,
        scale: placement.scale,
    };
    match world.get::<GlobalTransform>(entity) {
        Some(current) if *current == next => false,
        Some(_) => {
            if let Some(global) = world.get_mut::<GlobalTransform>(entity) {
                *global = next;
            }
            true
        }
        None => world.insert(entity, next),
    }
}

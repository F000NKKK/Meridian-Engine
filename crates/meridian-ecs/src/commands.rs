//! Structural changes recorded during iteration and applied afterwards.

use crate::{Component, Entity, World};

type Command = Box<dyn FnOnce(&mut World) + Send>;

/// A queue of structural changes to a [`World`]: spawn, despawn, insert and
/// remove. A query borrows the world for as long as it iterates, so a loop
/// cannot change it directly; it records what it wants here and the owner
/// calls [`Commands::apply`] once the loop is done.
///
/// Commands run in the order they were recorded. A command whose entity died
/// in the meantime does nothing, the same as calling the `World` method on a
/// dead entity.
#[derive(Default)]
pub struct Commands {
    queue: Vec<Command>,
}

impl Commands {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawns an entity when applied and hands it to `init`, which adds its
    /// components through the world it is given.
    pub fn spawn(&mut self, init: impl FnOnce(&mut World, Entity) + Send + 'static) {
        self.queue.push(Box::new(move |world| {
            let entity = world.spawn();
            init(world, entity);
        }));
    }

    /// Despawns `entity` when applied.
    pub fn despawn(&mut self, entity: Entity) {
        self.queue.push(Box::new(move |world| {
            world.despawn(entity);
        }));
    }

    /// Inserts `component` on `entity` when applied.
    pub fn insert<T: Component>(&mut self, entity: Entity, component: T) {
        self.queue.push(Box::new(move |world| {
            world.insert(entity, component);
        }));
    }

    /// Removes `T` from `entity` when applied.
    pub fn remove<T: Component>(&mut self, entity: Entity) {
        self.queue.push(Box::new(move |world| {
            world.remove::<T>(entity);
        }));
    }

    /// How many commands are waiting.
    #[must_use]
    pub fn len(&self) -> usize {
        self.queue.len()
    }

    /// Whether nothing is waiting.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Runs every recorded command against `world`, in order, and leaves the
    /// queue empty.
    pub fn apply(&mut self, world: &mut World) {
        for command in self.queue.drain(..) {
            command(world);
        }
    }
}

impl std::fmt::Debug for Commands {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Commands")
            .field("pending", &self.queue.len())
            .finish()
    }
}

//! Listing, spawning and despawning entities.

use meridian_ecs::Entity;
use meridian_editor_domain::{EntityRef, Event, RequestError, Response};

use crate::Game;
use crate::outcome::Outcome;

/// `ListEntities`: every live entity, ordered by index so lists are stable.
pub(crate) fn list(game: &Game<'_>) -> Outcome {
    let mut entities: Vec<EntityRef> = game.world.entities().map(EntityRef::from).collect();
    entities.sort_by_key(|e| (e.index, e.generation));
    Outcome::answer(Response::Entities { entities })
}

/// `SpawnEntity`: a new entity with no components.
pub(crate) fn spawn(game: &mut Game<'_>) -> Outcome {
    let entity = EntityRef::from(game.world.spawn());
    Outcome::answer(Response::Spawned { entity }).with_event(Event::EntityAdded { entity })
}

/// `DespawnEntity`.
pub(crate) fn despawn(game: &mut Game<'_>, entity: EntityRef) -> Outcome {
    if game.world.despawn(Entity::from(entity)) {
        Outcome::done().with_event(Event::EntityRemoved { entity })
    } else {
        Outcome::failed(RequestError::NoSuchEntity { entity })
    }
}

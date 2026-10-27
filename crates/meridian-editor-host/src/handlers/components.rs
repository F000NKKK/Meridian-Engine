//! Reading and writing one component of one entity.

use meridian_ecs_core::Entity;
use meridian_editor_protocol::{ComponentRef, EntityRef, Event, RequestError, Response};
use meridian_scene::ComponentEditError;
use substrate_data::Value;

use crate::Game;
use crate::outcome::Outcome;

/// `GetComponent`.
pub(crate) fn get(game: &Game<'_>, entity: EntityRef, component: &ComponentRef) -> Outcome {
    match read(game, entity, component) {
        Ok(value) => Outcome::answer(Response::Component { value }),
        Err(error) => Outcome::failed(error),
    }
}

/// `SetComponent`: replaces the component with `value`, read with the
/// component's own schema; announces the value as stored.
pub(crate) fn set(
    game: &mut Game<'_>,
    entity: EntityRef,
    component: ComponentRef,
    value: &Value,
) -> Outcome {
    let target = Entity::from(entity);
    let written = game
        .registry
        .set_component(game.world, target, &component, value);
    if let Err(error) = written {
        return Outcome::failed(edit_error(error, entity));
    }
    match read(game, entity, &component) {
        Ok(stored) => Outcome::done().with_event(Event::ComponentChanged {
            entity,
            component,
            value: stored,
        }),
        Err(error) => Outcome::failed(error),
    }
}

/// The component's value, or why there is none.
fn read(
    game: &Game<'_>,
    entity: EntityRef,
    component: &ComponentRef,
) -> Result<Value, RequestError> {
    let target = Entity::from(entity);
    if !game.world.is_alive(target) {
        return Err(RequestError::NoSuchEntity { entity });
    }
    let value = game
        .registry
        .component_value(game.world, target, component)
        .map_err(|error| edit_error(error, entity))?;
    value.ok_or_else(|| RequestError::ComponentMissing {
        entity,
        component: component.clone(),
    })
}

fn edit_error(error: ComponentEditError, entity: EntityRef) -> RequestError {
    match error {
        ComponentEditError::Unregistered { name } => {
            RequestError::NoSuchComponent { component: name }
        }
        ComponentEditError::NoSuchEntity => RequestError::NoSuchEntity { entity },
        ComponentEditError::Invalid { diagnostics } => RequestError::InvalidValue { diagnostics },
    }
}

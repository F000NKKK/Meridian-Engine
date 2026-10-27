//! [`Entry`]: one registered component.

use meridian_ecs::{Component, Entity, World};
use substrate_data::with::With;
use substrate_data::{Reader, Shape, Value, ValueRef};

use crate::ComponentName;

/// A read component, waiting for its entity: loading touches the `World`
/// only once the whole document has read without a problem.
pub(crate) type Insert = Box<dyn FnOnce(&mut World, Entity)>;

/// One registered component: its name, and its type's shape, read and write.
pub(crate) struct Entry {
    pub(crate) name: ComponentName,
    pub(crate) shape: fn() -> Shape,
    pub(crate) read: fn(&ValueRef<'_>, &mut Reader<'_>) -> Option<Insert>,
    pub(crate) write: fn(&World, Entity) -> Option<Value>,
}

impl Entry {
    /// The entry of `T` under `name`, read and written through `W`.
    pub(crate) fn of<T: Component, W: With<T>>(name: ComponentName) -> Self {
        Entry {
            name,
            shape: W::shape,
            read: read_component::<T, W>,
            write: write_component::<T, W>,
        }
    }
}

fn read_component<T: Component, W: With<T>>(
    v: &ValueRef<'_>,
    cx: &mut Reader<'_>,
) -> Option<Insert> {
    let component = W::read(v, cx)?;
    Some(Box::new(move |world: &mut World, entity| {
        world.insert(entity, component);
    }))
}

fn write_component<T: Component, W: With<T>>(world: &World, entity: Entity) -> Option<Value> {
    world.get::<T>(entity).map(W::write)
}

//! [`Entry`]: one registered component.

use meridian_ecs::{Component, Entity, World};
use substrate_data::with::With;
use substrate_data::{Reader, Shape, Value, ValueRef};

use super::Insert;
use super::read_component::read_with;
use crate::ComponentName;

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
    read_with(W::read(v, cx)?, W::write)
}

fn write_component<T: Component, W: With<T>>(world: &World, entity: Entity) -> Option<Value> {
    world.get::<T>(entity).map(W::write)
}

//! [`ReadComponent`]: a component read from a document, not inserted yet.

use meridian_ecs::{Component, Entity, World};
use substrate_data::Value;

/// A read component, waiting for its entity: loading touches the `World`
/// only once the whole document has read without a problem. It can also
/// say what it is as a document value, which is what a prefab instance's
/// field-level overrides are merged into.
pub(crate) trait ReadComponent {
    /// Gives the component to `entity`.
    fn insert(self: Box<Self>, world: &mut World, entity: Entity);
    /// The component as its document value (what saving it would write).
    fn document(&self) -> Value;
}

/// A component read, waiting to be inserted.
pub(crate) type Insert = Box<dyn ReadComponent>;

/// A `T` read through a codec, with the codec's way to write it back.
struct Read<T> {
    component: T,
    write: fn(&T) -> Value,
}

impl<T: Component> ReadComponent for Read<T> {
    fn insert(self: Box<Self>, world: &mut World, entity: Entity) {
        world.insert(entity, self.component);
    }

    fn document(&self) -> Value {
        (self.write)(&self.component)
    }
}

/// `component`, read through a codec whose way to write it back is `write`.
pub(super) fn read_with<T: Component>(component: T, write: fn(&T) -> Value) -> Option<Insert> {
    Some(Box::new(Read { component, write }))
}

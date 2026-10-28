//! [`SceneView`]: a world and the scene document that names its entities.

use meridian_ecs::World;

use crate::SceneMap;

/// What a save writes: the entities of `scene` as they are in `world`.
/// Entities of the world that `scene` does not list are not written.
#[derive(Debug, Clone, Copy)]
pub struct SceneView<'a> {
    /// The running world.
    pub world: &'a World,
    /// The document keys of the entities to write, as a load returned them.
    pub scene: &'a SceneMap,
}

impl<'a> SceneView<'a> {
    /// The listed entities of `world`.
    #[must_use]
    pub fn new(world: &'a World, scene: &'a SceneMap) -> Self {
        Self { world, scene }
    }
}

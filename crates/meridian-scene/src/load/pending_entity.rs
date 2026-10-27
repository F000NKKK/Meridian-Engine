//! [`PendingEntity`]: an entity read from a document, not spawned yet.

use meridian_ecs_core::World;

use crate::registry::Insert;
use crate::{DisplayName, EntityKey, KeptComponent, PrefabPath, SceneEntity};

/// An entity read from a document, not spawned yet.
pub(crate) struct PendingEntity {
    pub(crate) key: EntityKey,
    pub(crate) name: Option<DisplayName>,
    pub(crate) prefab: Option<PrefabPath>,
    pub(crate) parent: Option<EntityKey>,
    pub(crate) components: ComponentsDraft,
}

impl PendingEntity {
    /// Spawns it into `world`, with its components.
    pub(crate) fn spawn(self, world: &mut World) -> SceneEntity {
        let entity = world.spawn();
        for insert in self.components.inserts {
            insert(world, entity);
        }
        let mut e = SceneEntity::new(self.key, entity);
        e.name = self.name;
        e.prefab = self.prefab;
        e.parent = self.parent;
        e.kept = self.components.kept;
        e
    }
}

/// An entity's components as read: registered ones waiting for their
/// entity, unknown ones kept as written.
#[derive(Default)]
pub(crate) struct ComponentsDraft {
    pub(crate) inserts: Vec<Insert>,
    pub(crate) kept: Vec<KeptComponent>,
}

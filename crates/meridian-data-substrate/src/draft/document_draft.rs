//! [`DocumentDraft`]: a document read without a problem, not spawned yet.

use meridian_ecs::World;

use super::PendingEntity;
use crate::{DisplayName, Parent, SceneMap};

/// A document read without a problem, not spawned yet.
pub(crate) struct DocumentDraft {
    pub(crate) name: Option<DisplayName>,
    pub(crate) entities: Vec<PendingEntity>,
}

impl DocumentDraft {
    /// Spawns every entity, then links each to its parent.
    pub(crate) fn spawn(self, world: &mut World) -> SceneMap {
        let mut scene = SceneMap::new(self.name);
        for pending in self.entities {
            let entry = pending.spawn(world);
            scene
                .push(entry)
                .expect("entity keys are unique: duplicates are refused while reading");
        }
        for e in scene.entities() {
            if let Some(parent) = e.parent.as_ref().and_then(|k| scene.entity(k)) {
                world.insert(e.entity, Parent(parent));
            }
        }
        scene
    }
}

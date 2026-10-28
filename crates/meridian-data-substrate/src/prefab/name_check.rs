//! Checking the names that point into instances, once they are expanded.

use std::collections::HashSet;

use substrate_data::Path;

use crate::EntityKey;
use crate::document_keys::{ENTITIES, PARENT};
use crate::draft::PendingEntity;
use crate::load::SceneProblem;
use crate::read::{Finding, ReadContext};

impl ReadContext<'_> {
    /// A finding for every `@entity:` name below an instance that the
    /// expansion did not make, and for every instance whose parent is one
    /// of its own entities.
    pub(crate) fn check_names(&mut self, entities: &[PendingEntity]) -> Vec<Finding> {
        let keys: HashSet<&EntityKey> = entities.iter().map(|e| &e.key).collect();
        let mut found: Vec<Finding> = (self.pending.drain(..))
            .filter(|p| !keys.contains(&p.key))
            .map(|p| Finding {
                path: p.path,
                on_key: false,
                problem: SceneProblem::NoSuchGeneratedEntity {
                    suggestion: suggestion(&p.key, &p.instance, entities),
                    key: p.key,
                    instance: p.instance,
                },
            })
            .collect();
        found.extend(entities.iter().filter_map(|e| parent_inside(e, entities)));
        found
    }
}

/// The key under `instance` closest to `key`.
pub(super) fn suggestion(
    key: &EntityKey,
    instance: &EntityKey,
    entities: &[PendingEntity],
) -> Option<EntityKey> {
    let below = |e: &&PendingEntity| e.key == *instance || e.generated_by.is_some();
    let known = entities.iter().filter(below).map(|e| &e.key);
    key.closest(known).cloned()
}

/// A finding if instance `e` is parented to an entity its prefab made.
fn parent_inside(e: &PendingEntity, entities: &[PendingEntity]) -> Option<Finding> {
    e.prefab.as_ref()?;
    let parent = e.parent.as_ref()?;
    let inside = parent
        .as_str()
        .strip_prefix(e.key.as_str())?
        .starts_with('/');
    let made = entities
        .iter()
        .any(|g| &g.key == parent && g.generated_by.is_some());
    (inside && made).then(|| Finding {
        problem: SceneProblem::ParentInsideInstance {
            key: e.key.clone(),
            parent: parent.clone(),
        },
        path: Path::root().key(ENTITIES).index(e.index).key(PARENT),
        on_key: false,
    })
}

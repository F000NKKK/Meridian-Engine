//! [`ComponentDocs`]: an entity's components as document values.

use meridian_ecs::World;
use substrate_data::Value;

use super::TargetChanges;
use crate::draft::ComponentsDraft;
use crate::patch::patch_between;
use crate::{ComponentName, ComponentRegistry, SceneEntity};

/// The components of one entity, each as the value a document writes it
/// as: registered ones, then unknown ones kept as written.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ComponentDocs(Vec<(ComponentName, Value)>);

impl ComponentDocs {
    /// What `e` has in `world`.
    pub(crate) fn of_world(registry: &ComponentRegistry, world: &World, e: &SceneEntity) -> Self {
        let mut docs = Vec::new();
        for entry in registry.entries() {
            if let Some(v) = (entry.write)(world, e.entity) {
                docs.push((entry.name.clone(), v));
            }
        }
        let unknown = e.kept.iter().filter(|k| registry.entry(&k.name).is_none());
        docs.extend(unknown.map(|k| (k.name.clone(), k.value.clone())));
        ComponentDocs(docs)
    }

    /// What a draft entity will have.
    pub(crate) fn of_draft(draft: &ComponentsDraft) -> Self {
        let read = draft.inserts.iter().map(|(n, i)| (n.clone(), i.document()));
        let kept = draft.kept.iter().map(|k| (k.name.clone(), k.value.clone()));
        ComponentDocs(read.chain(kept).collect())
    }

    /// The components in document order.
    pub(crate) fn iter(&self) -> impl Iterator<Item = (&ComponentName, &Value)> {
        self.0.iter().map(|(n, v)| (n, v))
    }

    fn get(&self, name: &ComponentName) -> Option<&Value> {
        self.0.iter().find(|(n, _)| n == name).map(|(_, v)| v)
    }

    /// What it takes to turn `base` into these: the components `base`
    /// lacks or that can't be patched whole, the others as field patches,
    /// and the ones to take away.
    pub(crate) fn changes_from(&self, base: &ComponentDocs) -> TargetChanges {
        let mut changes = TargetChanges::default();
        for (name, now) in self.iter() {
            match base.get(name) {
                None => changes.replace.push((name.clone(), now.clone())),
                Some(was) if was == now => {}
                Some(was) => match patch_between(was, now) {
                    Some(patch) => changes.merge.push((name.clone(), patch)),
                    None => changes.replace.push((name.clone(), now.clone())),
                },
            }
        }
        let gone = base.iter().filter(|(name, _)| self.get(name).is_none());
        changes.remove.extend(gone.map(|(name, _)| name.clone()));
        changes
    }
}

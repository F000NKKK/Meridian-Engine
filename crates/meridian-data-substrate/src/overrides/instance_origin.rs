//! [`InstanceOrigin`]: what an instance's prefab makes, before any change.

use std::collections::HashMap;

use super::ComponentDocs;
use crate::draft::DocumentDraft;
use crate::{DisplayName, EntityKey};

/// The entities an instance's prefab makes, as they would load with no
/// `components` or `overrides` on the instance: what saving compares the
/// world's entities with.
#[derive(Debug)]
pub(crate) struct InstanceOrigin {
    /// The instance root's name from the prefab.
    pub(crate) name: Option<DisplayName>,
    entities: HashMap<EntityKey, ComponentDocs>,
}

impl InstanceOrigin {
    /// The base of the instance `draft` holds (its root first).
    pub(crate) fn of_draft(draft: &DocumentDraft) -> Self {
        let name = draft.entities.first().and_then(|root| root.name.clone());
        let docs = |e: &crate::draft::PendingEntity| {
            (e.key.clone(), ComponentDocs::of_draft(&e.components))
        };
        InstanceOrigin {
            name,
            entities: draft.entities.iter().map(docs).collect(),
        }
    }

    /// The components of the entity keyed `key`, if the prefab makes it.
    pub(crate) fn components(&self, key: &EntityKey) -> Option<&ComponentDocs> {
        self.entities.get(key)
    }
}

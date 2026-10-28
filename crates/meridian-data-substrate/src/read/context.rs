//! [`ReadContext`]: what reading a document needs from the caller.

use substrate_data::{Path, Resolve, ValueRef};

use super::document_reader::DocumentReader;
use super::names::PendingRef;
use super::reading::Reading;
use super::{DocumentRole, Finding};
use crate::document_keys::{ENTITIES, OVERRIDES};
use crate::draft::{DocumentDraft, PendingEntity};
use crate::load::SceneProblem;
use crate::prefab::PrefabSource;
use crate::{ComponentRegistry, PrefabPath};

/// What reading a document needs: the registry, the caller's resolver for
/// references that are not entities, and, when prefabs are instanced, their
/// source and the chain of prefabs being read (to find cycles).
pub(crate) struct ReadContext<'a> {
    pub(crate) registry: &'a ComponentRegistry,
    pub(crate) resolver: &'a mut dyn Resolve,
    pub(crate) prefabs: Option<&'a mut dyn PrefabSource>,
    pub(crate) chain: Vec<PrefabPath>,
    /// References into instances of the document being read, to check once
    /// they are expanded.
    pub(crate) pending: Vec<PendingRef>,
}

impl<'a> ReadContext<'a> {
    /// A context that leaves `prefab` fields alone (only checked by `resolver`).
    pub(crate) fn new(registry: &'a ComponentRegistry, resolver: &'a mut dyn Resolve) -> Self {
        ReadContext {
            registry,
            resolver,
            prefabs: None,
            chain: Vec::new(),
            pending: Vec::new(),
        }
    }

    /// The same, instancing every `prefab` field from `prefabs`.
    pub(crate) fn instancing(mut self, prefabs: &'a mut dyn PrefabSource) -> Self {
        self.prefabs = Some(prefabs);
        self
    }

    /// Reads the document `root`, then instances its prefabs when this
    /// context does. The draft is there only if nothing is wrong.
    pub(crate) fn read(&mut self, role: &DocumentRole, root: &ValueRef<'_>) -> Reading {
        let mut reading = DocumentReader::read((self.registry, role), root, self.resolver);
        let pending = std::mem::take(&mut reading.pending);
        let outer = std::mem::replace(&mut self.pending, pending);
        if let Some(draft) = reading.draft.take() {
            match self.expanded(draft) {
                Ok(draft) => reading.draft = Some(draft),
                Err(found) => reading.findings.extend(found),
            }
        }
        self.pending = outer;
        reading
    }

    /// `draft` with its prefabs instanced (when there is a source for
    /// them), or every problem with that or with the names that point into
    /// the instances.
    fn expanded(&mut self, draft: DocumentDraft) -> Result<DocumentDraft, Vec<Finding>> {
        let DocumentDraft { name, entities } = draft;
        let entities = if self.prefabs.is_some() {
            self.expand(entities)?
        } else {
            refuse_overrides(&entities)?;
            entities
        };
        let found = self.check_names(&entities);
        if found.is_empty() {
            Ok(DocumentDraft { name, entities })
        } else {
            Err(found)
        }
    }
}

/// Without prefabs, `overrides` have nothing to override.
fn refuse_overrides(entities: &[PendingEntity]) -> Result<(), Vec<Finding>> {
    let refused: Vec<Finding> = (entities.iter())
        .filter(|e| !e.overrides.is_empty())
        .map(|e| Finding {
            problem: SceneProblem::OverridesNeedPrefabs { key: e.key.clone() },
            path: Path::root().key(ENTITIES).index(e.index).key(OVERRIDES),
            on_key: true,
        })
        .collect();
    if refused.is_empty() {
        Ok(())
    } else {
        Err(refused)
    }
}

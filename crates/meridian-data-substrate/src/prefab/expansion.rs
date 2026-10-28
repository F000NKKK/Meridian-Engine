//! Expanding the `prefab` fields of a document's entities.

use std::collections::HashMap;

use substrate_data::Path;

use super::PrefabInstance;
use crate::document_keys::{ENTITIES, ID, PREFAB};
use crate::draft::{ComponentsDraft, DocumentDraft, PendingEntity};
use crate::load::{LoadError, SceneIssue, SceneProblem};
use crate::read::{DocumentRole, DocumentSource, Finding, ReadContext};
use crate::{EntityKey, PrefabPath};

/// How deep prefabs may include prefabs. A prefab that includes itself is
/// refused sooner, as a cycle; this bounds a chain that never repeats.
pub const MAX_PREFAB_DEPTH: usize = 16;

impl ReadContext<'_> {
    /// `entities` with every one that says `prefab` replaced by its
    /// instance: the entity itself as the root, then the prefab's other
    /// entities. Every prefab is read, so a problem in any of them (or one
    /// that is missing, cyclic, too deep, or makes a key twice) fails the
    /// whole expansion.
    pub(crate) fn expand(
        &mut self,
        entities: Vec<PendingEntity>,
    ) -> Result<Vec<PendingEntity>, Vec<Finding>> {
        let mut expanded = Vec::with_capacity(entities.len());
        let mut found = Vec::new();
        for entity in entities {
            let Some(prefab) = entity.prefab.clone() else {
                expanded.push(entity);
                continue;
            };
            match self.instance(entity, &prefab) {
                Ok(group) => expanded.extend(group),
                Err(problems) => found.extend(problems),
            }
        }
        found.extend(repeated_keys(&expanded));
        if found.is_empty() {
            Ok(expanded)
        } else {
            Err(found)
        }
    }

    /// `instance` of a prefab made as if a scene said so: its entities,
    /// root first, nothing spawned.
    pub(crate) fn instanced(
        &mut self,
        instance: &PrefabInstance,
    ) -> Result<DocumentDraft, LoadError> {
        let root = PendingEntity {
            index: 0,
            key: instance.key.clone(),
            name: None,
            prefab: Some(instance.prefab.clone()),
            parent: None,
            components: ComponentsDraft::default(),
            overrides: Vec::new(),
            generated_by: None,
        };
        match self.expand(vec![root]) {
            Ok(entities) => Ok(DocumentDraft {
                name: None,
                entities,
            }),
            Err(found) => Err(unplaced(found)),
        }
    }

    /// The entities `entity` becomes, saying `prefab`: itself, then what
    /// the prefab makes, with the entity's `overrides` applied.
    fn instance(
        &mut self,
        mut entity: PendingEntity,
        prefab: &PrefabPath,
    ) -> Result<Vec<PendingEntity>, Vec<Finding>> {
        let at = entity.index;
        let read = self.prefab_draft(&entity.key, prefab);
        let Some(draft) = read.map_err(|problem| vec![at_prefab(problem, at)])? else {
            return Ok(vec![entity]);
        };
        let overrides = std::mem::take(&mut entity.overrides);
        let mut group = entity.instanced(draft.entities);
        let found = self.apply_overrides(&mut group, overrides);
        if found.is_empty() {
            Ok(group)
        } else {
            Err(found)
        }
    }

    /// The entities of `prefab`, read to be instanced as `instance`; none
    /// when there is no source of prefabs.
    fn prefab_draft(
        &mut self,
        instance: &EntityKey,
        prefab: &PrefabPath,
    ) -> Result<Option<DocumentDraft>, SceneProblem> {
        if self.chain.contains(prefab) {
            let mut chain = self.chain.clone();
            chain.push(prefab.clone());
            return Err(SceneProblem::PrefabCycle { chain });
        }
        if self.chain.len() >= MAX_PREFAB_DEPTH {
            return Err(SceneProblem::PrefabTooDeep {
                limit: MAX_PREFAB_DEPTH,
            });
        }
        let Some(source) = self.prefabs.as_deref_mut() else {
            return Ok(None);
        };
        let bytes = source
            .prefab(prefab)
            .map_err(|u| SceneProblem::PrefabUnavailable {
                prefab: prefab.clone(),
                reason: u.reason,
                suggestion: u.suggestion,
            })?;
        self.chain.push(prefab.clone());
        let role = DocumentRole::Instance(instance.clone());
        let read = DocumentSource::detect(&bytes).and_then(|s| s.draft(&role, self));
        self.chain.pop();
        match read {
            Ok(draft) => Ok(Some(draft)),
            Err(error) => Err(SceneProblem::PrefabInvalid {
                prefab: prefab.clone(),
                error: Box::new(error),
            }),
        }
    }
}

/// `problem` at the `prefab` field of entity `index`.
fn at_prefab(problem: SceneProblem, index: usize) -> Finding {
    Finding {
        problem,
        path: Path::root().key(ENTITIES).index(index).key(PREFAB),
        on_key: false,
    }
}

/// A finding for every key an expansion makes that another entity has.
fn repeated_keys(entities: &[PendingEntity]) -> Vec<Finding> {
    let mut first: HashMap<&EntityKey, usize> = HashMap::new();
    let mut found = Vec::new();
    for e in entities {
        match first.get(&e.key) {
            Some(&first) => {
                let problem = SceneProblem::DuplicateEntity {
                    key: e.key.clone(),
                    first,
                };
                found.push(Finding {
                    problem,
                    path: key_path(e),
                    on_key: false,
                });
            }
            None => {
                first.insert(&e.key, e.index);
            }
        }
    }
    found
}

/// Where entity `e` says its key: its `id`, or for one an instance made,
/// the `prefab` field of the entity that made it.
fn key_path(e: &PendingEntity) -> Path {
    let entity = Path::root().key(ENTITIES).index(e.index);
    if e.generated_by.is_some() {
        entity.key(PREFAB)
    } else {
        entity.key(ID)
    }
}

/// The findings of a standalone instance, which has no document: paths only.
fn unplaced(found: Vec<Finding>) -> LoadError {
    LoadError::new(
        found
            .into_iter()
            .map(|f| SceneIssue::unplaced(f.problem, f.path))
            .collect(),
    )
}

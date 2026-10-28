//! [`SceneProblem`]: what can be wrong with a scene document.

use std::fmt;

use substrate_data::Problem;

use super::LoadError;
use crate::{ComponentName, ComponentNameError, EntityKey, EntityKeyError, PrefabPath};

/// What can be wrong with a scene document.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum SceneProblem {
    /// A problem with a value, as Substrate Data finds it (syntax, a wrong
    /// type, a missing field, an `@entity:` key the document hasn't, …).
    Data {
        /// Which problem.
        problem: Problem,
        /// What's wrong, for a person.
        message: String,
    },
    /// A `format` this crate doesn't read.
    UnsupportedFormat {
        /// The format written.
        found: u32,
    },
    /// An entity key used twice.
    DuplicateEntity {
        /// The key.
        key: EntityKey,
        /// The index of its first entity.
        first: usize,
    },
    /// An entity that names itself as its parent.
    OwnParent {
        /// Its key.
        key: EntityKey,
    },
    /// A component name the registry doesn't know.
    UnknownComponent {
        /// The name.
        name: ComponentName,
        /// The registered name it is probably a typo of.
        suggestion: Option<ComponentName>,
    },
    /// A `components` key that isn't a component name at all.
    BadComponentName(ComponentNameError),
    /// A prefab with no entity, so no root.
    PrefabEmpty,
    /// A prefab whose first entity, the root, has a parent.
    PrefabRootHasParent {
        /// The root's key.
        key: EntityKey,
    },
    /// A prefab entity other than the root without a parent: a prefab is a
    /// tree under its root.
    PrefabOrphan {
        /// Its key.
        key: EntityKey,
    },
    /// A prefab its source could not give.
    PrefabUnavailable {
        /// The prefab asked for.
        prefab: PrefabPath,
        /// Why, for a person.
        reason: String,
        /// The path of a prefab there is that this is probably a typo of.
        suggestion: Option<PrefabPath>,
    },
    /// A prefab that, through the prefabs it instances, instances itself.
    PrefabCycle {
        /// The prefabs being read, outermost first, ending with the repeat.
        chain: Vec<PrefabPath>,
    },
    /// Prefabs inside prefabs deeper than [`crate::MAX_PREFAB_DEPTH`].
    PrefabTooDeep {
        /// The depth allowed.
        limit: usize,
    },
    /// A prefab that does not load: every problem in it, as it would be
    /// reported for a document of its own.
    PrefabInvalid {
        /// The prefab.
        prefab: PrefabPath,
        /// Its problems.
        error: Box<LoadError>,
    },
    /// An `overrides` key that isn't an entity key at all.
    BadOverrideTarget(EntityKeyError),
    /// An override aimed at an entity that is neither the instance nor
    /// below it (`<instance>/…`).
    OverrideOutsideInstance {
        /// The target as the document wrote it.
        target: EntityKey,
        /// The instance the overrides are in.
        instance: EntityKey,
    },
    /// An entity below an instance that its prefab doesn't make, named by
    /// an override, a `parent` or a reference.
    NoSuchGeneratedEntity {
        /// The entity named, as keyed.
        key: EntityKey,
        /// The instance it was looked for under.
        instance: EntityKey,
        /// A key the instance does make that this is probably a typo of.
        suggestion: Option<EntityKey>,
    },
    /// `overrides` on an entity that doesn't say `prefab`.
    OverridesWithoutPrefab {
        /// The entity.
        key: EntityKey,
    },
    /// `overrides` in a document read without a prefab source, which can't
    /// know what they override.
    OverridesNeedPrefabs {
        /// The instance.
        key: EntityKey,
    },
    /// A `remove` naming a component its target doesn't have.
    OverrideRemovesMissing {
        /// The target, as keyed.
        target: EntityKey,
        /// The component.
        name: ComponentName,
    },
    /// An instance whose `parent` is an entity its own prefab makes.
    ParentInsideInstance {
        /// The instance.
        key: EntityKey,
        /// Its parent, below it.
        parent: EntityKey,
    },
}

impl fmt::Display for SceneProblem {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SceneProblem::Data { message, .. } => f.write_str(message),
            SceneProblem::UnsupportedFormat { found } => {
                write!(
                    f,
                    "unsupported scene format {found} (this reads {})",
                    crate::FORMAT
                )
            }
            SceneProblem::DuplicateEntity { key, first } => {
                write!(f, "duplicate entity `{key}` (first at entities[{first}])")
            }
            SceneProblem::OwnParent { key } => write!(f, "entity `{key}` can't be its own parent"),
            SceneProblem::UnknownComponent {
                name,
                suggestion: Some(s),
            } => {
                write!(f, "unknown component `{name}` — did you mean `{s}`?")
            }
            SceneProblem::UnknownComponent {
                name,
                suggestion: None,
            } => write!(f, "unknown component `{name}`"),
            SceneProblem::BadComponentName(e) => e.fmt(f),
            SceneProblem::PrefabEmpty => f.write_str("a prefab needs at least its root entity"),
            SceneProblem::PrefabRootHasParent { key } => {
                write!(f, "the root of a prefab (`{key}`) can't have a parent")
            }
            SceneProblem::PrefabOrphan { key } => {
                write!(
                    f,
                    "prefab entity `{key}` needs a parent (only the root has none)"
                )
            }
            SceneProblem::PrefabUnavailable {
                prefab,
                reason,
                suggestion,
            } => {
                write!(f, "prefab `{prefab}`: {reason}")?;
                match suggestion {
                    Some(s) => write!(f, " — did you mean `{s}`?"),
                    None => Ok(()),
                }
            }
            SceneProblem::PrefabCycle { chain } => {
                let chain: Vec<&str> = chain.iter().map(PrefabPath::as_str).collect();
                write!(f, "prefabs include each other: {}", chain.join(" -> "))
            }
            SceneProblem::PrefabTooDeep { limit } => {
                write!(f, "prefabs inside prefabs nest deeper than {limit}")
            }
            SceneProblem::PrefabInvalid { prefab, error } => {
                let inner = error.to_string().replace('\n', "\n  ");
                write!(f, "prefab `{prefab}` doesn't load:\n  {inner}")
            }
            SceneProblem::BadOverrideTarget(e) => e.fmt(f),
            SceneProblem::OverrideOutsideInstance { target, instance } => write!(
                f,
                "override target `{target}` is not `{instance}` or an entity below it"
            ),
            SceneProblem::NoSuchGeneratedEntity {
                key,
                instance,
                suggestion,
            } => {
                write!(f, "the prefab of `{instance}` makes no entity `{key}`")?;
                match suggestion {
                    Some(s) => write!(f, " — did you mean `{s}`?"),
                    None => Ok(()),
                }
            }
            SceneProblem::OverridesWithoutPrefab { key } => {
                write!(f, "entity `{key}` has `overrides` but no `prefab`")
            }
            SceneProblem::OverridesNeedPrefabs { key } => write!(
                f,
                "`{key}` has `overrides`, which need its prefab: load with a prefab source"
            ),
            SceneProblem::OverrideRemovesMissing { target, name } => {
                write!(f, "`{target}` has no component `{name}` to remove")
            }
            SceneProblem::ParentInsideInstance { key, parent } => write!(
                f,
                "instance `{key}` can't have its own `{parent}` as its parent"
            ),
        }
    }
}

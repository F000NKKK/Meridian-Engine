//! [`SceneProblem`]: what can be wrong with a scene document.

use std::fmt;

use substrate_data::Problem;

use crate::{ComponentName, ComponentNameError, EntityKey};

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
        }
    }
}

//! [`ComponentEditError`]: why one component could not be read or written.

use substrate_data::Diagnostic;

use crate::ComponentName;

/// Why one component of one entity could not be read or written through
/// the registry.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum ComponentEditError {
    /// No component is registered under the name.
    #[error("no component is registered as `{name}`")]
    Unregistered {
        /// The name.
        name: ComponentName,
    },
    /// The entity is not alive in the world.
    #[error("the entity is not alive")]
    NoSuchEntity,
    /// The value is not of the component's shape.
    #[error("the value is not a valid component ({} problem(s))", .diagnostics.len())]
    Invalid {
        /// Every problem, with its path in the value.
        diagnostics: Vec<Diagnostic>,
    },
}

//! [`BindError`]: why a key could not be bound.

use thiserror::Error;

/// Why [`ResourceIdTable::bind`](super::ResourceIdTable::bind) refused.
/// The table is unchanged after any of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum BindError {
    /// The key was never declared (it has no `.meta`).
    #[error("the key is not known to the table")]
    UnknownKey,
    /// The key already has a resource; release it first.
    #[error("the key is already bound to a resource")]
    AlreadyBound,
    /// The resource is already the identity of another key.
    #[error("the resource already belongs to another key")]
    ResourceInUse,
}

//! [`Binding`]: whether a known key has a runtime identity yet.

use crate::ResourceId;

/// What a [`ResourceIdTable`](super::ResourceIdTable) knows about a key.
pub enum Binding<T> {
    /// The key is known but nothing has been loaded for it (or the
    /// resource was released).
    Unbound,
    /// The key's resource lives under this identity.
    Bound(ResourceId<T>),
}

impl<T> Clone for Binding<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for Binding<T> {}

impl<T> PartialEq for Binding<T> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Binding::Unbound, Binding::Unbound) => true,
            (Binding::Bound(a), Binding::Bound(b)) => a == b,
            _ => false,
        }
    }
}
impl<T> Eq for Binding<T> {}

impl<T> std::fmt::Debug for Binding<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Binding::Unbound => f.write_str("Unbound"),
            Binding::Bound(id) => f.debug_tuple("Bound").field(id).finish(),
        }
    }
}

impl<T> Binding<T> {
    /// The identity, if bound.
    #[must_use]
    pub fn id(self) -> Option<ResourceId<T>> {
        match self {
            Binding::Bound(id) => Some(id),
            Binding::Unbound => None,
        }
    }
}

//! [`ResourceDependency`]: one declared edge between two resources.

use crate::ResourceId;

/// A declared dependency between two resources, e.g. a material depending
/// on the texture it references.
///
/// `Clone`/`Copy`/`Debug`/`PartialEq` are implemented manually for the same
/// reason as on [`ResourceId`]: a `#[derive]` here would require `A: Trait`
/// and `B: Trait`, which is wrong — the type parameters only ever appear
/// inside `ResourceId<A>`/`ResourceId<B>`, both of which are already
/// unconditionally `Copy`.
pub struct ResourceDependency<A, B> {
    /// The dependent resource.
    pub from: ResourceId<A>,
    /// The resource it needs.
    pub on: ResourceId<B>,
}

impl<A, B> ResourceDependency<A, B> {
    /// `from` depends on `on`.
    #[must_use]
    pub fn new(from: ResourceId<A>, on: ResourceId<B>) -> Self {
        Self { from, on }
    }
}

impl<A, B> Clone for ResourceDependency<A, B> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<A, B> Copy for ResourceDependency<A, B> {}

impl<A, B> PartialEq for ResourceDependency<A, B> {
    fn eq(&self, other: &Self) -> bool {
        self.from == other.from && self.on == other.on
    }
}
impl<A, B> Eq for ResourceDependency<A, B> {}

impl<A, B> core::fmt::Debug for ResourceDependency<A, B> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ResourceDependency")
            .field("from", &self.from)
            .field("on", &self.on)
            .finish()
    }
}

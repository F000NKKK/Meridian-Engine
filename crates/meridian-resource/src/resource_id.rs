//! [`ResourceId`]: a typed identity for one resource kind.

use core::marker::PhantomData;

use meridian_memory::Handle;

/// A typed identity for a specific resource kind, e.g. `ResourceId<Texture>`.
/// Wraps a `meridian_memory::Handle`; the type parameter exists purely
/// to keep a `TextureHandle` and a `MeshHandle` from being interchangeable.
///
/// `Clone`/`Copy`/`Debug` are implemented manually rather than derived so
/// that `T` itself is never required to implement them (a `#[derive]` here
/// would wrongly add that bound just because of the `PhantomData<T>` field).
pub struct ResourceId<T> {
    /// The pool handle this identity wraps.
    pub handle: Handle,
    _marker: PhantomData<fn() -> T>,
}

impl<T> ResourceId<T> {
    /// The identity of the `T` resource at `handle`.
    #[must_use]
    pub fn new(handle: Handle) -> Self {
        Self {
            handle,
            _marker: PhantomData,
        }
    }
}

impl<T> Clone for ResourceId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for ResourceId<T> {}

impl<T> PartialEq for ResourceId<T> {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}
impl<T> Eq for ResourceId<T> {}

impl<T> core::fmt::Debug for ResourceId<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ResourceId")
            .field("handle", &self.handle)
            .finish()
    }
}

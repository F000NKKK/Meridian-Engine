//! Resource identity: typed handles, versioning, lifetime and dependency tracking for textures/meshes/buffers/shaders. Not a manager — the type system resource lifecycle is built on.

use core::marker::PhantomData;
use meridian_memory_core::Handle;

/// A typed identity for a specific resource kind, e.g. `ResourceId<Texture>`.
/// Wraps a `meridian_memory_core::Handle`; the type parameter exists purely
/// to keep a `TextureHandle` and a `MeshHandle` from being interchangeable.
///
/// `Clone`/`Copy`/`Debug` are implemented manually rather than derived so
/// that `T` itself is never required to implement them (a `#[derive]` here
/// would wrongly add that bound just because of the `PhantomData<T>` field).
pub struct ResourceId<T> {
    pub handle: Handle,
    _marker: PhantomData<fn() -> T>,
}

impl<T> ResourceId<T> {
    pub fn new(handle: Handle) -> Self {
        Self { handle, _marker: PhantomData }
    }
}

impl<T> Clone for ResourceId<T> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<T> Copy for ResourceId<T> {}

impl<T> core::fmt::Debug for ResourceId<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ResourceId").field("handle", &self.handle).finish()
    }
}

/// Monotonically increasing version, bumped whenever a resource is reloaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Version(pub u32);

/// A declared dependency between two resources, e.g. a material depending
/// on the texture it references.
pub struct ResourceDependency<A, B> {
    pub from: ResourceId<A>,
    pub on: ResourceId<B>,
}

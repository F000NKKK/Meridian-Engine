//! Resource identity: typed handles, versioning, lifetime and dependency tracking for textures/meshes/buffers/shaders. Not a manager — the type system resource lifecycle is built on.
//!
//! Deliberately does not load, cache, or evict anything (see ADR 006 /
//! docs/dependency-rules.md rule 8) — [`DependencyGraph`] tracks the
//! topology of declared dependencies between resources a caller already
//! owns elsewhere; it never owns a resource's data or decides when it's
//! freed.

use core::marker::PhantomData;
use std::collections::HashMap;

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

/// Monotonically increasing version, bumped whenever a resource is reloaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Version(pub u32);

impl Version {
    /// The version after this one. Wrapping is intentional: a resource
    /// reloaded `u32::MAX` times is already a bug worth finding elsewhere,
    /// not a reason to panic here.
    pub fn next(self) -> Version {
        Version(self.0.wrapping_add(1))
    }
}

/// A declared dependency between two resources, e.g. a material depending
/// on the texture it references.
///
/// `Clone`/`Copy`/`Debug`/`PartialEq` are implemented manually for the same
/// reason as on [`ResourceId`]: a `#[derive]` here would require `A: Trait`
/// and `B: Trait`, which is wrong — the type parameters only ever appear
/// inside `ResourceId<A>`/`ResourceId<B>`, both of which are already
/// unconditionally `Copy`.
pub struct ResourceDependency<A, B> {
    pub from: ResourceId<A>,
    pub on: ResourceId<B>,
}

impl<A, B> ResourceDependency<A, B> {
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

/// Tracks the topology of declared [`ResourceDependency`] edges, keyed by
/// the underlying `Handle` (type-erasing `A`/`B` — the graph reasons about
/// "does this handle transitively depend on that one," not about what kind
/// of resource either is). Deliberately does not store resource data: this
/// is a graph over handles the caller already owns elsewhere, not a pool.
#[derive(Debug, Default)]
pub struct DependencyGraph {
    edges: HashMap<Handle, Vec<Handle>>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Records that `dep.from` depends on `dep.on`. Idempotent: recording
    /// the same edge twice doesn't duplicate it.
    pub fn add_dependency<A, B>(&mut self, dep: ResourceDependency<A, B>) {
        let targets = self.edges.entry(dep.from.handle).or_default();
        if !targets.contains(&dep.on.handle) {
            targets.push(dep.on.handle);
        }
    }

    /// The handles `from` directly depends on (not transitive).
    pub fn direct_dependencies(&self, from: Handle) -> &[Handle] {
        self.edges.get(&from).map(Vec::as_slice).unwrap_or(&[])
    }

    /// True if `from` depends on `on`, directly or transitively.
    pub fn depends_on(&self, from: Handle, on: Handle) -> bool {
        let mut stack = vec![from];
        let mut visited = std::collections::HashSet::new();
        while let Some(current) = stack.pop() {
            if !visited.insert(current) {
                continue;
            }
            for &next in self.direct_dependencies(current) {
                if next == on {
                    return true;
                }
                stack.push(next);
            }
        }
        false
    }

    /// True if recording a dependency from `from` on `on` would close a
    /// cycle (including the trivial `from == on` self-dependency) — check
    /// this before [`add_dependency`](Self::add_dependency) if the graph
    /// must stay acyclic (e.g. before wiring a material to a texture, to
    /// catch "texture's shader references this same material" upstream).
    pub fn would_cycle(&self, from: Handle, on: Handle) -> bool {
        from == on || self.depends_on(on, from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Texture;
    struct Material;
    struct Mesh;

    fn handle(index: u32) -> Handle {
        Handle {
            index,
            generation: 0,
        }
    }

    #[test]
    fn version_next_increments() {
        assert_eq!(Version(0).next(), Version(1));
        assert_eq!(Version(41).next(), Version(42));
    }

    #[test]
    fn version_next_wraps_instead_of_panicking() {
        assert_eq!(Version(u32::MAX).next(), Version(0));
    }

    #[test]
    fn resource_id_equality_is_by_handle() {
        let a: ResourceId<Texture> = ResourceId::new(handle(1));
        let b: ResourceId<Texture> = ResourceId::new(handle(1));
        let c: ResourceId<Texture> = ResourceId::new(handle(2));
        assert_eq!(a, b);
        assert_ne!(a, c);
    }

    #[test]
    fn dependency_graph_tracks_direct_dependency() {
        let mut graph = DependencyGraph::new();
        let material: ResourceId<Material> = ResourceId::new(handle(1));
        let texture: ResourceId<Texture> = ResourceId::new(handle(2));

        graph.add_dependency(ResourceDependency::new(material, texture));

        assert!(graph.depends_on(material.handle, texture.handle));
        assert_eq!(
            graph.direct_dependencies(material.handle),
            &[texture.handle]
        );
    }

    #[test]
    fn adding_the_same_dependency_twice_does_not_duplicate() {
        let mut graph = DependencyGraph::new();
        let material: ResourceId<Material> = ResourceId::new(handle(1));
        let texture: ResourceId<Texture> = ResourceId::new(handle(2));

        graph.add_dependency(ResourceDependency::new(material, texture));
        graph.add_dependency(ResourceDependency::new(material, texture));

        assert_eq!(graph.direct_dependencies(material.handle).len(), 1);
    }

    #[test]
    fn dependency_is_transitive() {
        // material -> texture -> ... a "mesh depends on material depends on texture" chain.
        let mut graph = DependencyGraph::new();
        let mesh: ResourceId<Mesh> = ResourceId::new(handle(1));
        let material: ResourceId<Material> = ResourceId::new(handle(2));
        let texture: ResourceId<Texture> = ResourceId::new(handle(3));

        graph.add_dependency(ResourceDependency::new(mesh, material));
        graph.add_dependency(ResourceDependency::new(material, texture));

        assert!(graph.depends_on(mesh.handle, texture.handle));
        assert!(!graph.depends_on(texture.handle, mesh.handle));
    }

    #[test]
    fn unrelated_resources_do_not_depend_on_each_other() {
        let mut graph = DependencyGraph::new();
        let a: ResourceId<Texture> = ResourceId::new(handle(1));
        let b: ResourceId<Texture> = ResourceId::new(handle(2));
        let unrelated: ResourceId<Texture> = ResourceId::new(handle(3));

        graph.add_dependency(ResourceDependency::new(a, b));

        assert!(!graph.depends_on(a.handle, unrelated.handle));
        assert!(!graph.depends_on(unrelated.handle, a.handle));
    }

    #[test]
    fn would_cycle_detects_self_dependency() {
        let graph = DependencyGraph::new();
        let a: ResourceId<Texture> = ResourceId::new(handle(1));
        assert!(graph.would_cycle(a.handle, a.handle));
    }

    #[test]
    fn would_cycle_detects_indirect_cycle() {
        // a -> b -> c already exists; adding c -> a would close the loop.
        let mut graph = DependencyGraph::new();
        let a: ResourceId<Texture> = ResourceId::new(handle(1));
        let b: ResourceId<Texture> = ResourceId::new(handle(2));
        let c: ResourceId<Texture> = ResourceId::new(handle(3));

        graph.add_dependency(ResourceDependency::new(a, b));
        graph.add_dependency(ResourceDependency::new(b, c));

        assert!(graph.would_cycle(c.handle, a.handle));
        assert!(
            !graph.would_cycle(a.handle, c.handle),
            "a -> c doesn't close a cycle, it's already implied"
        );
    }
}

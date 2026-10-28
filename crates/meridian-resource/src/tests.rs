//! Identity, versions and the dependency graph.

use meridian_memory::Handle;

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

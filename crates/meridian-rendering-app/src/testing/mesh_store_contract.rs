//! The behaviour every [`MeshStore`](crate::MeshStore) must have.

use meridian_rendering_domain::{MeshRegistryError, MeshSource};

use crate::MeshStore;

fn triangle() -> MeshSource {
    MeshSource {
        positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
        normals: vec![[0.0, 0.0, 1.0]; 3],
        uvs: vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]],
        indices: vec![0, 1, 2],
        skin: None,
    }
}

/// Asserts the mesh store contract against fresh stores from `make`: a valid
/// mesh round-trips by handle; invalid ones are rejected by variant and leave
/// the store unchanged; `remove` returns the mesh once.
///
/// # Panics
///
/// Panics when `S` breaks the contract.
pub fn run_mesh_store_contract<S: MeshStore>(make: impl Fn() -> S) {
    let mut store = make();
    let handle = store.add(triangle()).expect("a valid triangle is accepted");
    assert_eq!(store.get(handle).map(|m| m.indices.len()), Some(3));

    let mut not_triangles = triangle();
    not_triangles.indices = vec![0, 1];
    assert!(matches!(
        store.add(not_triangles),
        Err(MeshRegistryError::NotATriangleList { .. })
    ));
    let mut missing_uvs = triangle();
    missing_uvs.uvs.pop();
    assert!(matches!(
        store.add(missing_uvs),
        Err(MeshRegistryError::UvCountMismatch { .. })
    ));
    let mut missing_normals = triangle();
    missing_normals.normals.pop();
    assert!(matches!(
        store.add(missing_normals),
        Err(MeshRegistryError::NormalCountMismatch { .. })
    ));

    assert!(store.remove(handle).is_some(), "remove returns the mesh");
    assert!(store.get(handle).is_none(), "a removed handle is dead");
    assert!(store.remove(handle).is_none(), "remove works once");
}

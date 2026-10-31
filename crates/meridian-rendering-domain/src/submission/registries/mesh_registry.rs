//! Registered meshes by handle.

use meridian_memory::ResourcePool;

use crate::MeshHandle;
use crate::submission::{MeshRegistryError, MeshSource};

/// Handle-addressed storage for [`MeshSource`](crate::submission::MeshSource)s — the CPU-side registry
/// [`Renderable3d::mesh`](crate::scene::Renderable3d::mesh) resolves against. Owns no GPU objects; GPU
/// buffers are built fresh per instance in `bake_batches` (in `meridian-rendering-wgpu`)
/// (see the module doc).
#[derive(Debug, Default)]
pub struct MeshRegistry {
    sources: ResourcePool<MeshSource>,
}

impl MeshRegistry {
    /// Creates an empty mesh registry.
    pub fn new() -> Self {
        Self::default()
    }

    /// Validates and stores mesh geometry, returning its stable handle.
    pub fn register(&mut self, source: MeshSource) -> Result<MeshHandle, MeshRegistryError> {
        if !source.indices.len().is_multiple_of(3) {
            return Err(MeshRegistryError::NotATriangleList {
                index_count: source.indices.len(),
            });
        }
        if source.uvs.len() != source.positions.len() {
            return Err(MeshRegistryError::UvCountMismatch {
                positions: source.positions.len(),
                uvs: source.uvs.len(),
            });
        }
        if source.normals.len() != source.positions.len() {
            return Err(MeshRegistryError::NormalCountMismatch {
                positions: source.positions.len(),
                normals: source.normals.len(),
            });
        }
        let vertex_count = source.positions.len();
        if let Some(skin) = &source.skin {
            let skinned = skin.joints.len().max(skin.weights.len());
            if skin.joints.len() != vertex_count || skin.weights.len() != vertex_count {
                return Err(MeshRegistryError::SkinCountMismatch {
                    positions: vertex_count,
                    skinned,
                });
            }
        }
        check_morph(&source, vertex_count)?;
        if let Some(&index) = source
            .indices
            .iter()
            .find(|&&index| index as usize >= vertex_count)
        {
            return Err(MeshRegistryError::IndexOutOfRange {
                index,
                vertex_count,
            });
        }
        Ok(MeshHandle::new(self.sources.insert(source)))
    }

    /// Removes and returns the mesh for `handle`, if still live.
    pub fn unregister(&mut self, handle: MeshHandle) -> Option<MeshSource> {
        self.sources.remove(handle.handle)
    }

    /// Returns the mesh source for a live handle.
    pub fn get(&self, handle: MeshHandle) -> Option<&MeshSource> {
        self.sources.get(handle.handle)
    }
}

/// Every morph target gives every vertex one offset.
fn check_morph(source: &MeshSource, vertex_count: usize) -> Result<(), MeshRegistryError> {
    let wrong = source
        .morph
        .iter()
        .flat_map(|morph| &morph.targets)
        .flat_map(|target| {
            let normals = (!target.normals.is_empty()).then_some(target.normals.len());
            [Some(target.positions.len()), normals]
        })
        .flatten()
        .find(|&offsets| offsets != vertex_count);
    match wrong {
        Some(offsets) => Err(MeshRegistryError::MorphCountMismatch {
            positions: vertex_count,
            offsets,
        }),
        None => Ok(()),
    }
}

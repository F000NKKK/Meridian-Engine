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
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(&mut self, source: MeshSource) -> Result<MeshHandle, MeshRegistryError> {
        if source.positions.len() > u16::MAX as usize {
            return Err(MeshRegistryError::TooManyVertices {
                count: source.positions.len(),
            });
        }
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
        Ok(MeshHandle::new(self.sources.insert(source)))
    }

    pub fn unregister(&mut self, handle: MeshHandle) -> Option<MeshSource> {
        self.sources.remove(handle.handle)
    }

    pub fn get(&self, handle: MeshHandle) -> Option<&MeshSource> {
        self.sources.get(handle.handle)
    }
}

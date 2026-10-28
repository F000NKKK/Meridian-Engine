//! The GPU copies of registered meshes.

use std::collections::HashMap;

use meridian_memory::Handle;
use meridian_rendering_domain::{MeshHandle, MeshRegistry, MeshSource};

use super::gpu_mesh::GpuMesh;
use crate::Device;

/// Uploads each registered mesh once, on first use, and keeps it until the
/// registry drops it. The registry is immutable per handle, so a cached
/// copy never goes stale; a reused slot has a new generation and so a new
/// key.
#[derive(Debug, Default)]
pub(crate) struct MeshCache {
    meshes: HashMap<Handle, GpuMesh>,
}

impl MeshCache {
    /// Uploads `source` unless `handle` is already cached.
    pub(crate) fn ensure(&mut self, device: &Device, handle: MeshHandle, source: &MeshSource) {
        self.meshes
            .entry(handle.handle)
            .or_insert_with(|| GpuMesh::upload(device, source));
    }

    /// The cached copy of `handle`'s mesh.
    pub(crate) fn get(&self, handle: MeshHandle) -> Option<&GpuMesh> {
        self.meshes.get(&handle.handle)
    }

    /// Drops copies of meshes that are no longer registered.
    pub(crate) fn retain_registered(&mut self, registry: &MeshRegistry) {
        self.meshes
            .retain(|&handle, _| registry.get(MeshHandle::new(handle)).is_some());
    }
}

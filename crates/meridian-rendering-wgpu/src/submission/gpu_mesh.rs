//! A mesh uploaded once and drawn by any number of instances.

use meridian_rendering_domain::MeshSource;

use crate::{Buffer, BufferUsage, Device};

/// A mesh's vertex and index buffers: local-space data, shared by every
/// instance and every frame until the mesh is unregistered.
#[derive(Debug)]
pub(crate) struct GpuMesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
}

impl GpuMesh {
    /// Uploads `source`: position, normal and UV per vertex, `u16` indices
    /// (the registry rejects meshes with more vertices than that).
    pub(crate) fn upload(device: &Device, source: &MeshSource) -> Self {
        let vertices = source
            .positions
            .iter()
            .zip(&source.normals)
            .zip(&source.uvs)
            .flat_map(|((position, normal), uv)| position.iter().chain(normal).chain(uv))
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<u8>>();
        let indices = source
            .indices
            .iter()
            .flat_map(|&index| u16::try_from(index).unwrap_or(0).to_le_bytes())
            .collect::<Vec<u8>>();
        let vertex_buffer = device.create_buffer(vertices.len(), BufferUsage::Vertex);
        device.write_buffer(&vertex_buffer, &vertices);
        let index_buffer = device.create_buffer(indices.len(), BufferUsage::Index);
        device.write_buffer(&index_buffer, &indices);
        Self {
            vertex_buffer,
            index_buffer,
            index_count: u32::try_from(source.indices.len()).unwrap_or(u32::MAX),
        }
    }
}

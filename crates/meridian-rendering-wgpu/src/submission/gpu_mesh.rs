//! A mesh uploaded once and drawn by any number of instances.

use meridian_rendering_domain::MeshSource;

use super::skinning::{GpuMorph, GpuSkin};
use crate::{Buffer, BufferUsage, Device};

/// A mesh's vertex and index buffers: local-space data, shared by every
/// instance and every frame until the mesh is unregistered.
#[derive(Debug)]
pub(crate) struct GpuMesh {
    pub vertex_buffer: Buffer,
    pub index_buffer: Buffer,
    pub index_count: u32,
    /// The joints and weights of a skinned mesh (ADR 047).
    pub skin: Option<GpuSkin>,
    /// The blend shapes of a morphed mesh (ADR 047).
    pub morph: Option<GpuMorph>,
    pub vertex_count: u32,
}

impl GpuMesh {
    /// Uploads `source`: position, normal and UV per vertex and `u32` indices
    /// (the registry has checked that each names a vertex).
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
            .flat_map(|index| index.to_le_bytes())
            .collect::<Vec<u8>>();
        // A skinned or morphed mesh's base is also read by the pose kernel.
        let usage = if source.skin.is_some() || source.morph.is_some() {
            BufferUsage::VertexStorage
        } else {
            BufferUsage::Vertex
        };
        let vertex_buffer = device.create_buffer(vertices.len(), usage);
        device.write_buffer(&vertex_buffer, &vertices);
        let index_buffer = device.create_buffer(indices.len(), BufferUsage::Index);
        device.write_buffer(&index_buffer, &indices);
        Self {
            vertex_buffer,
            index_buffer,
            index_count: u32::try_from(source.indices.len()).unwrap_or(u32::MAX),
            skin: source
                .skin
                .as_ref()
                .map(|skin| GpuSkin::upload(device, skin)),
            morph: source
                .morph
                .as_ref()
                .map(|morph| GpuMorph::upload(device, morph, source.positions.len())),
            vertex_count: u32::try_from(source.positions.len()).unwrap_or(u32::MAX),
        }
    }
}

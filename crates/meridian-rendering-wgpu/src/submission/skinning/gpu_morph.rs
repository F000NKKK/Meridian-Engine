//! A mesh's morph target offsets on the GPU.

use meridian_rendering_domain::MeshMorph;

use crate::{Buffer, BufferUsage, Device};

/// The offsets of every target for every vertex, uploaded once with the mesh.
#[derive(Debug)]
pub(crate) struct GpuMorph {
    /// Six `f32` per vertex per target, target after target: the position
    /// offset then the normal offset (zero where the target has none).
    pub(crate) offsets: Buffer,
    pub(crate) targets: u32,
}

impl GpuMorph {
    pub(crate) fn upload(device: &Device, morph: &MeshMorph, vertices: usize) -> Self {
        let mut bytes = Vec::with_capacity(morph.targets.len() * vertices * 24);
        for target in &morph.targets {
            for vertex in 0..vertices {
                let position = target.positions.get(vertex).copied().unwrap_or_default();
                let normal = target.normals.get(vertex).copied().unwrap_or_default();
                bytes.extend(position.iter().chain(&normal).flat_map(|v| v.to_le_bytes()));
            }
        }
        let offsets = device.create_buffer(bytes.len(), BufferUsage::Storage);
        device.write_buffer(&offsets, &bytes);
        Self {
            offsets,
            targets: u32::try_from(morph.targets.len()).unwrap_or(u32::MAX),
        }
    }
}

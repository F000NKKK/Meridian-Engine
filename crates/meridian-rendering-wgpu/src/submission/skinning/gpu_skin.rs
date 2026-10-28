//! A skinned mesh's per-vertex joints and weights on the GPU.

use meridian_rendering_domain::VertexSkin;

use crate::{Buffer, BufferUsage, Device};

/// The joints and weights of each vertex, uploaded once with the mesh.
#[derive(Debug)]
pub(crate) struct GpuSkin {
    /// Four `u32` joint indices per vertex.
    pub(crate) joints: Buffer,
    /// Four `f32` weights per vertex.
    pub(crate) weights: Buffer,
}

impl GpuSkin {
    pub(crate) fn upload(device: &Device, skin: &VertexSkin) -> Self {
        let joints: Vec<u8> = skin
            .joints
            .iter()
            .flatten()
            .flat_map(|&joint| u32::from(joint).to_le_bytes())
            .collect();
        let weights: Vec<u8> = skin
            .weights
            .iter()
            .flatten()
            .flat_map(|weight| weight.to_le_bytes())
            .collect();
        let joint_buffer = device.create_buffer(joints.len(), BufferUsage::Storage);
        device.write_buffer(&joint_buffer, &joints);
        let weight_buffer = device.create_buffer(weights.len(), BufferUsage::Storage);
        device.write_buffer(&weight_buffer, &weights);
        Self {
            joints: joint_buffer,
            weights: weight_buffer,
        }
    }
}

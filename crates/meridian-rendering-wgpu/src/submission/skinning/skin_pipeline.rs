//! The compute pipeline that poses skinned and morphed meshes.

use std::sync::Arc;

use meridian_rendering_domain::{JointPalette, MeshHandle};

use super::SkinJob;
use super::mesh_pose::MeshPose;
use super::skin_shader::SKIN_SHADER_WGSL;
use super::skin_slot::SkinSlot;
use super::skin_slot::SlotSizes;
use crate::submission::gpu_mesh::GpuMesh;
use crate::{Buffer, BufferUsage, ComputePipeline, Device};

/// Vertices one workgroup of the kernel poses.
const WORKGROUP_VERTICES: u32 = 64;

/// `u32` slots in front of the weights in a control block.
const CONTROL_HEADER: usize = 4;

/// Builds the dispatches that pose skinned and morphed instances, keeping
/// each instance's buffers from one frame to the next (ADR 047).
#[derive(Debug)]
pub(crate) struct SkinPipeline {
    pipeline: ComputePipeline,
    pub(super) slots: Vec<SkinSlot>,
    /// How many slots this frame's posed instances have taken so far.
    used: usize,
    /// Stands in for the bindings a mesh without a skin or morph does not
    /// have: the kernel never reads past what its control block says.
    unused: Buffer,
}

impl SkinPipeline {
    pub(crate) fn new(device: &Device) -> Self {
        let shader = device.create_shader("meridian-skin", SKIN_SHADER_WGSL);
        let unused = device.create_buffer(64, BufferUsage::Storage);
        device.write_buffer(&unused, &[0; 64]);
        Self {
            pipeline: device.create_compute_pipeline(&shader, "skin"),
            slots: Vec::new(),
            used: 0,
            unused,
        }
    }

    /// The pipeline the jobs dispatch against.
    pub(crate) fn pipeline(&self) -> &ComputePipeline {
        &self.pipeline
    }

    /// Starts a frame's posed instances over from the first slot.
    pub(crate) fn begin_frame(&mut self) {
        self.used = 0;
    }

    /// Frees the slots this frame's instances did not take.
    pub(crate) fn end_frame(&mut self) {
        self.slots.truncate(self.used);
    }

    /// Poses `mesh` (registered as `handle`) with `palette` and the morph
    /// `weights`: writes them into the next slot, making the slot afresh
    /// when the mesh or the sizes are not the ones the slot had. `None` when
    /// nothing poses the mesh: no usable palette for a skin and no morph
    /// targets for the weights.
    pub(crate) fn job(
        &mut self,
        device: &Device,
        handle: MeshHandle,
        mesh: &GpuMesh,
        pose: MeshPose<'_>,
    ) -> Option<SkinJob> {
        let matrices = pose
            .palette
            .filter(|_| mesh.skin.is_some())
            .map(matrix_bytes)
            .filter(|bytes| !bytes.is_empty());
        let targets = mesh.morph.as_ref().map_or(0, |morph| morph.targets);
        if mesh.vertex_count == 0 || (matrices.is_none() && targets == 0) {
            return None;
        }
        let control = control_bytes(targets, mesh.vertex_count, matrices.is_some(), pose.weights);
        let at = self.used;
        let sizes = SlotSizes {
            palette: matrices.as_ref().map(Vec::len),
            control: control.len(),
        };
        if !self
            .slots
            .get(at)
            .is_some_and(|slot| slot.fits(handle, sizes))
        {
            let slot = self.slot_for(device, handle, mesh, sizes);
            if at < self.slots.len() {
                self.slots[at] = slot;
            } else {
                self.slots.push(slot);
            }
        }
        self.used += 1;
        let slot = &self.slots[at];
        if let (Some(buffer), Some(matrices)) = (&slot.palette, &matrices) {
            device.write_buffer(buffer, matrices);
        }
        device.write_buffer(&slot.control, &control);
        Some(SkinJob {
            bind_group: slot.bind_group.clone(),
            workgroups: mesh.vertex_count.div_ceil(WORKGROUP_VERTICES),
            posed: slot.posed.clone(),
        })
    }

    /// New buffers of `sizes` for `mesh`.
    fn slot_for(
        &self,
        device: &Device,
        handle: MeshHandle,
        mesh: &GpuMesh,
        sizes: SlotSizes,
    ) -> SkinSlot {
        let palette = sizes
            .palette
            .map(|bytes| device.create_buffer(bytes, BufferUsage::Storage));
        let control = device.create_buffer(sizes.control, BufferUsage::Storage);
        let posed = device.create_buffer(mesh.vertex_buffer.byte_len, BufferUsage::VertexStorage);
        let (joints, weights) = mesh
            .skin
            .as_ref()
            .map_or((&self.unused, &self.unused), |skin| {
                (&skin.joints, &skin.weights)
            });
        let offsets = mesh
            .morph
            .as_ref()
            .map_or(&self.unused, |morph| &morph.offsets);
        let bind_group = device.create_compute_bind_group(
            &self.pipeline,
            &[
                &mesh.vertex_buffer,
                joints,
                weights,
                palette.as_ref().unwrap_or(&self.unused),
                &posed,
                offsets,
                &control,
            ],
        );
        SkinSlot {
            mesh: handle,
            palette,
            control,
            posed: Arc::new(posed),
            bind_group: Arc::new(bind_group),
        }
    }
}

fn matrix_bytes(palette: &JointPalette) -> Vec<u8> {
    palette
        .matrices()
        .iter()
        .flatten()
        .flatten()
        .flat_map(|value| value.to_le_bytes())
        .collect()
}

/// The kernel's control block: targets, vertices, whether to skin and a
/// spare, then one weight per target (zero where `weights` is short).
fn control_bytes(targets: u32, vertices: u32, skinned: bool, weights: &[f32]) -> Vec<u8> {
    let mut bytes: Vec<u8> = [targets, vertices, u32::from(skinned), 0]
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect();
    let count = targets as usize;
    bytes.reserve(count * 4);
    for target in 0..count {
        let weight = weights.get(target).copied().unwrap_or(0.0);
        bytes.extend(weight.to_le_bytes());
    }
    debug_assert_eq!(bytes.len(), (CONTROL_HEADER + count) * 4);
    bytes
}

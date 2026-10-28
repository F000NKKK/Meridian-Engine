//! The compute pipeline that poses skinned meshes.

use std::sync::Arc;

use meridian_rendering_domain::{JointPalette, MeshHandle};

use super::SkinJob;
use super::skin_shader::SKIN_SHADER_WGSL;
use super::skin_slot::SkinSlot;
use crate::submission::gpu_mesh::GpuMesh;
use crate::{Buffer, BufferUsage, ComputePipeline, Device};

/// Vertices one workgroup of the kernel poses.
const WORKGROUP_VERTICES: u32 = 64;

/// Builds the dispatches that pose skinned instances, keeping each
/// instance's buffers from one frame to the next (ADR 047).
#[derive(Debug)]
pub(crate) struct SkinPipeline {
    pipeline: ComputePipeline,
    slots: Vec<SkinSlot>,
    /// How many slots this frame's skinned instances have taken so far.
    used: usize,
}

impl SkinPipeline {
    pub(crate) fn new(device: &Device) -> Self {
        let shader = device.create_shader("meridian-skin", SKIN_SHADER_WGSL);
        Self {
            pipeline: device.create_compute_pipeline(&shader, "skin"),
            slots: Vec::new(),
            used: 0,
        }
    }

    /// The pipeline the jobs dispatch against.
    pub(crate) fn pipeline(&self) -> &ComputePipeline {
        &self.pipeline
    }

    /// Starts a frame's skinned instances over from the first slot.
    pub(crate) fn begin_frame(&mut self) {
        self.used = 0;
    }

    /// Frees the slots this frame's instances did not take.
    pub(crate) fn end_frame(&mut self) {
        self.slots.truncate(self.used);
    }

    /// Poses `mesh` (registered as `handle`) with `palette`: writes the
    /// palette into the next slot, making the slot afresh when the mesh or
    /// the palette's size is not the one the slot had. `None` when the mesh
    /// has no skin or the palette is empty.
    pub(crate) fn job(
        &mut self,
        device: &Device,
        handle: MeshHandle,
        mesh: &GpuMesh,
        palette: &JointPalette,
    ) -> Option<SkinJob> {
        let skin = mesh.skin.as_ref()?;
        if palette.matrices().is_empty() || skin.vertex_count == 0 {
            return None;
        }
        let matrices: Vec<u8> = palette
            .matrices()
            .iter()
            .flatten()
            .flatten()
            .flat_map(|value| value.to_le_bytes())
            .collect();
        let at = self.used;
        if !self
            .slots
            .get(at)
            .is_some_and(|slot| slot.fits(handle, matrices.len()))
        {
            let inputs = [&mesh.vertex_buffer, &skin.joints, &skin.weights];
            let slot = self.slot_for(device, handle, inputs, matrices.len());
            if at < self.slots.len() {
                self.slots[at] = slot;
            } else {
                self.slots.push(slot);
            }
        }
        self.used += 1;
        let slot = &self.slots[at];
        device.write_buffer(&slot.palette, &matrices);
        Some(SkinJob {
            bind_group: slot.bind_group.clone(),
            workgroups: skin.vertex_count.div_ceil(WORKGROUP_VERTICES),
            posed: slot.posed.clone(),
        })
    }

    /// New buffers for `mesh` posed by a palette of `palette_bytes` bytes.
    fn slot_for(
        &self,
        device: &Device,
        handle: MeshHandle,
        [vertices, joints, weights]: [&Buffer; 3],
        palette_bytes: usize,
    ) -> SkinSlot {
        let palette = device.create_buffer(palette_bytes, BufferUsage::Storage);
        let posed = device.create_buffer(vertices.byte_len, BufferUsage::VertexStorage);
        let bind_group = device.create_compute_bind_group(
            &self.pipeline,
            &[vertices, joints, weights, &palette, &posed],
        );
        SkinSlot {
            mesh: handle,
            palette,
            posed: Arc::new(posed),
            bind_group: Arc::new(bind_group),
        }
    }
}

#[cfg(all(test, feature = "hw-tests"))]
mod tests {
    use meridian_rendering_domain::{MeshSource, VertexSkin};

    use super::*;

    fn translation(x: f32) -> [[f32; 4]; 4] {
        [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [x, 0.0, 0.0, 1.0],
        ]
    }

    fn handle() -> MeshHandle {
        MeshHandle::new(meridian_memory::Handle::default())
    }

    /// A triangle whose vertices follow joint 0 fully, joint 1 fully, and
    /// both halves.
    fn skinned_triangle() -> MeshSource {
        MeshSource {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            normals: vec![[0.0, 0.0, 1.0]; 3],
            uvs: vec![[0.25, 0.75]; 3],
            indices: vec![0, 1, 2],
            skin: Some(VertexSkin {
                joints: vec![[0, 0, 0, 0], [1, 0, 0, 0], [0, 1, 0, 0]],
                weights: vec![
                    [1.0, 0.0, 0.0, 0.0],
                    [1.0, 0.0, 0.0, 0.0],
                    [0.5, 0.5, 0.0, 0.0],
                ],
            }),
        }
    }

    #[tokio::test]
    async fn the_kernel_moves_vertices_by_their_weighted_joints_and_keeps_uvs() {
        let Ok(device) = Device::new().await else {
            eprintln!("skipping: no GPU device available");
            return;
        };
        let mesh = GpuMesh::upload(&device, &skinned_triangle());
        let palette = JointPalette::new(vec![translation(10.0), translation(20.0)]);
        let mut pipeline = SkinPipeline::new(&device);
        let job = pipeline
            .job(&device, handle(), &mesh, &palette)
            .expect("a skinned mesh");

        let mut commands = device.create_command_buffer();
        commands.dispatch_compute(pipeline.pipeline(), &job.bind_group, job.workgroups);
        commands.inner.submit();
        let bytes = device.read_buffer(&job.posed).await;
        let floats: Vec<f32> = bytes
            .chunks_exact(4)
            .map(|chunk| f32::from_le_bytes(chunk.try_into().expect("four bytes")))
            .collect();
        // Vertex 0 follows joint 0, vertex 1 joint 1, vertex 2 both halves.
        assert_eq!(&floats[0..3], &[10.0, 0.0, 0.0]);
        assert_eq!(&floats[8..11], &[21.0, 0.0, 0.0]);
        assert_eq!(&floats[16..19], &[15.0, 1.0, 0.0]);
        assert_eq!(
            &floats[3..6],
            &[0.0, 0.0, 1.0],
            "a translation keeps normals"
        );
        assert_eq!(&floats[6..8], &[0.25, 0.75]);
    }

    #[tokio::test]
    async fn a_rigid_mesh_or_an_empty_palette_makes_no_job() {
        let Ok(device) = Device::new().await else {
            return;
        };
        let mut pipeline = SkinPipeline::new(&device);
        let skinned = GpuMesh::upload(&device, &skinned_triangle());
        assert!(
            pipeline
                .job(&device, handle(), &skinned, &JointPalette::new(Vec::new()))
                .is_none()
        );
        let rigid = GpuMesh::upload(
            &device,
            &MeshSource {
                skin: None,
                ..skinned_triangle()
            },
        );
        let palette = JointPalette::new(vec![translation(1.0)]);
        assert!(pipeline.job(&device, handle(), &rigid, &palette).is_none());
    }

    #[tokio::test]
    async fn a_slot_is_reused_next_frame_until_the_palette_changes_size() {
        let Ok(device) = Device::new().await else {
            return;
        };
        let mut pipeline = SkinPipeline::new(&device);
        let mesh = GpuMesh::upload(&device, &skinned_triangle());
        let two = JointPalette::new(vec![translation(1.0), translation(2.0)]);
        let three = JointPalette::new(vec![translation(1.0); 3]);

        pipeline.begin_frame();
        let first = pipeline.job(&device, handle(), &mesh, &two).expect("a job");
        pipeline.end_frame();
        pipeline.begin_frame();
        let again = pipeline.job(&device, handle(), &mesh, &two).expect("a job");
        pipeline.end_frame();
        assert!(Arc::ptr_eq(&first.posed, &again.posed), "same buffers");

        pipeline.begin_frame();
        let grown = pipeline
            .job(&device, handle(), &mesh, &three)
            .expect("a job");
        pipeline.end_frame();
        assert!(
            !Arc::ptr_eq(&first.posed, &grown.posed),
            "a new palette size"
        );

        pipeline.begin_frame();
        pipeline.end_frame();
        assert!(pipeline.slots.is_empty(), "unused slots are freed");
    }
}

//! The compute pipeline that poses skinned meshes.

use std::sync::Arc;

use meridian_rendering_domain::JointPalette;

use super::SkinJob;
use super::skin_shader::SKIN_SHADER_WGSL;
use crate::submission::gpu_mesh::GpuMesh;
use crate::{BufferUsage, ComputePipeline, Device};

/// Vertices one workgroup of the kernel poses.
const WORKGROUP_VERTICES: u32 = 64;

/// Builds the dispatches that pose skinned instances.
#[derive(Debug)]
pub(crate) struct SkinPipeline {
    pipeline: ComputePipeline,
}

impl SkinPipeline {
    pub(crate) fn new(device: &Device) -> Self {
        let shader = device.create_shader("meridian-skin", SKIN_SHADER_WGSL);
        Self {
            pipeline: device.create_compute_pipeline(&shader, "skin"),
        }
    }

    /// The pipeline the jobs dispatch against.
    pub(crate) fn pipeline(&self) -> &ComputePipeline {
        &self.pipeline
    }

    /// Poses `mesh` with `palette`: a fresh output buffer and the dispatch
    /// that fills it. `None` when the mesh has no skin or the palette is empty.
    pub(crate) fn job(
        &self,
        device: &Device,
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
        let palette_buffer = device.create_buffer(matrices.len(), BufferUsage::Storage);
        device.write_buffer(&palette_buffer, &matrices);
        let posed = device.create_buffer(mesh.vertex_buffer.byte_len, BufferUsage::VertexStorage);
        let bind_group = device.create_compute_bind_group(
            &self.pipeline,
            &[
                &mesh.vertex_buffer,
                &skin.joints,
                &skin.weights,
                &palette_buffer,
                &posed,
            ],
        );
        Some(SkinJob {
            bind_group,
            workgroups: skin.vertex_count.div_ceil(WORKGROUP_VERTICES),
            posed: Arc::new(posed),
        })
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
        let pipeline = SkinPipeline::new(&device);
        let job = pipeline
            .job(&device, &mesh, &palette)
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
        let pipeline = SkinPipeline::new(&device);
        let skinned = GpuMesh::upload(&device, &skinned_triangle());
        assert!(
            pipeline
                .job(&device, &skinned, &JointPalette::new(Vec::new()))
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
        assert!(pipeline.job(&device, &rigid, &palette).is_none());
    }
}

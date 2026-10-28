//! Posing on a real device (needs the `hw-tests` feature).

use std::sync::Arc;

use meridian_rendering_domain::{
    JointPalette, MeshHandle, MeshMorph, MeshSource, MorphTarget, VertexSkin,
};

use super::{MeshPose, SkinJob, SkinPipeline};
use crate::Device;
use crate::submission::gpu_mesh::GpuMesh;

fn pose<'a>(palette: Option<&'a JointPalette>, weights: &'a [f32]) -> MeshPose<'a> {
    MeshPose { palette, weights }
}

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
        morph: None,
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
        .job(&device, handle(), &mesh, pose(Some(&palette), &[]))
        .expect("a skinned mesh");

    let mut commands = device.create_command_buffer();
    commands.dispatch_compute(pipeline.pipeline(), &job.bind_group, job.workgroups);
    commands.inner.submit();
    let bytes = device.read_buffer(&job.posed).await;
    let floats: Vec<f32> = bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
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
            .job(
                &device,
                handle(),
                &skinned,
                pose(Some(&JointPalette::new(Vec::new())), &[]),
            )
            .is_none()
    );
    let rigid = GpuMesh::upload(
        &device,
        &MeshSource {
            skin: None,
            morph: None,
            ..skinned_triangle()
        },
    );
    let palette = JointPalette::new(vec![translation(1.0)]);
    assert!(
        pipeline
            .job(&device, handle(), &rigid, pose(Some(&palette), &[]))
            .is_none()
    );
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
    let first = pipeline
        .job(&device, handle(), &mesh, pose(Some(&two), &[]))
        .expect("a job");
    pipeline.end_frame();
    pipeline.begin_frame();
    let again = pipeline
        .job(&device, handle(), &mesh, pose(Some(&two), &[]))
        .expect("a job");
    pipeline.end_frame();
    assert!(Arc::ptr_eq(&first.posed, &again.posed), "same buffers");

    pipeline.begin_frame();
    let grown = pipeline
        .job(&device, handle(), &mesh, pose(Some(&three), &[]))
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

async fn posed_floats(device: &Device, pipeline: &SkinPipeline, job: &SkinJob) -> Vec<f32> {
    let mut commands = device.create_command_buffer();
    commands.dispatch_compute(pipeline.pipeline(), &job.bind_group, job.workgroups);
    commands.inner.submit();
    device
        .read_buffer(&job.posed)
        .await
        .as_chunks::<4>()
        .0
        .iter()
        .map(|chunk| f32::from_le_bytes(*chunk))
        .collect()
}

/// Two targets: one lifts every vertex by 2 on y (and tilts normals to y),
/// one pushes vertex 0 by 4 on x.
fn morphed_triangle() -> MeshSource {
    MeshSource {
        skin: None,
        morph: Some(MeshMorph {
            targets: vec![
                MorphTarget {
                    positions: vec![[0.0, 2.0, 0.0]; 3],
                    normals: vec![[0.0, 1.0, 0.0]; 3],
                },
                MorphTarget {
                    positions: vec![[4.0, 0.0, 0.0], [0.0; 3], [0.0; 3]],
                    normals: Vec::new(),
                },
            ],
        }),
        ..skinned_triangle()
    }
}

#[tokio::test]
async fn weighted_targets_add_their_offsets_and_a_missing_weight_is_zero() {
    let Ok(device) = Device::new().await else {
        return;
    };
    let mesh = GpuMesh::upload(&device, &morphed_triangle());
    let mut pipeline = SkinPipeline::new(&device);
    let job = pipeline
        .job(&device, handle(), &mesh, pose(None, &[0.5]))
        .expect("a morphed mesh");
    let floats = posed_floats(&device, &pipeline, &job).await;
    assert_eq!(&floats[0..3], &[0.0, 1.0, 0.0], "half the first target");
    assert_eq!(&floats[8..11], &[1.0, 1.0, 0.0]);
    // The normal (0, 0, 1) plus half of (0, 1, 0), renormalised.
    let length = 1.25f32.sqrt();
    assert!((floats[4] - 0.5 / length).abs() < 1e-5);
    assert!((floats[5] - 1.0 / length).abs() < 1e-5);
    assert_eq!(&floats[6..8], &[0.25, 0.75]);
}

#[tokio::test]
async fn morphing_comes_before_skinning() {
    let Ok(device) = Device::new().await else {
        return;
    };
    let source = MeshSource {
        morph: morphed_triangle().morph,
        ..skinned_triangle()
    };
    let mesh = GpuMesh::upload(&device, &source);
    let palette = JointPalette::new(vec![translation(10.0), translation(20.0)]);
    let mut pipeline = SkinPipeline::new(&device);
    let job = pipeline
        .job(&device, handle(), &mesh, pose(Some(&palette), &[1.0, 1.0]))
        .expect("a posed mesh");
    let floats = posed_floats(&device, &pipeline, &job).await;
    // Vertex 0: base (0,0,0) + (0,2,0) + (4,0,0), then joint 0 adds 10.
    assert_eq!(&floats[0..3], &[14.0, 2.0, 0.0]);
    // Vertex 1 follows joint 1: (1,0,0) + (0,2,0), then +20.
    assert_eq!(&floats[8..11], &[21.0, 2.0, 0.0]);
}

#[tokio::test]
async fn a_mesh_with_neither_a_palette_nor_targets_makes_no_job() {
    let Ok(device) = Device::new().await else {
        return;
    };
    let mesh = GpuMesh::upload(&device, &morphed_triangle());
    let rigid = GpuMesh::upload(
        &device,
        &MeshSource {
            morph: None,
            ..morphed_triangle()
        },
    );
    let mut pipeline = SkinPipeline::new(&device);
    assert!(
        pipeline
            .job(&device, handle(), &rigid, pose(None, &[1.0]))
            .is_none()
    );
    assert!(
        pipeline
            .job(&device, handle(), &mesh, pose(None, &[]))
            .is_some(),
        "targets at weight zero still pose (the caller skips them)"
    );
}

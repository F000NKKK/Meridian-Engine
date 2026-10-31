//! A real GPU guards skipping empty bloom without retaining a previous frame's glow.

use meridian_gac::{Motor3, Projection, Scale3, Vec3};
use meridian_rendering_app::{FrameOutcome, FrameRecorder, FrameRequest};
use meridian_rendering_domain::{
    Material, MaterialHandle, MaterialRegistry, MeshHandle, MeshRegistry, MeshSource, Renderable3d,
    Scene3d,
};

use crate::{Device, GpuFrameRecorder, Surface};

#[tokio::test]
async fn bloom_tracks_emission_and_does_not_reuse_a_previous_glow() {
    let device = Device::new().await.expect("a real GPU adapter is required");
    let surface = Surface::offscreen(&device, 64, 48);
    let mut recorder = GpuFrameRecorder::new(device, surface, 64, 48);
    let timed = recorder.enable_profiling();
    let mut meshes = MeshRegistry::new();
    let mesh = meshes.register(triangle()).unwrap();
    let mut materials = MaterialRegistry::new();
    let plain = Material {
        base_color_factor: [0.1, 0.1, 0.1, 1.0],
        unlit: true,
        ..Material::default()
    };
    let dark = materials.register(plain);
    let glowing = materials.register(Material {
        emissive: [4.0, 0.0, 0.0],
        ..plain
    });
    let mut scene = scene(mesh, dark);
    let mut frames = Vec::new();
    for (material, emits) in [(dark, false), (glowing, true), (dark, false)] {
        scene.renderables[0].material = material;
        assert_eq!(
            recorder.record_frame(&FrameRequest {
                scene: &scene,
                meshes: &meshes,
                materials: &materials,
                overlays: &[],
                clear_color: [0.0, 0.0, 0.0, 1.0],
            }),
            FrameOutcome::Presented
        );
        frames.push(recorder.read_pixels().expect("real pixels"));
        if timed {
            let has_bloom = recorder
                .pass_timings()
                .iter()
                .any(|timing| timing.label.starts_with("bloom"));
            assert_eq!(has_bloom, emits);
        }
    }
    assert!(frames[0] != frames[1], "emissive pixels must be visible");
    assert_eq!(
        frames[0], frames[2],
        "no previous glow remains after emission is removed"
    );
}

fn triangle() -> MeshSource {
    MeshSource {
        positions: vec![[-0.5, -0.5, -2.0], [0.5, -0.5, -2.0], [0.0, 0.5, -2.0]],
        normals: vec![[0.0, 0.0, 1.0]; 3],
        uvs: vec![[0.0; 2]; 3],
        indices: vec![0, 1, 2],
        skin: None,
        morph: None,
    }
}

fn scene(mesh: MeshHandle, material: MaterialHandle) -> Scene3d {
    let mut scene = Scene3d {
        camera: meridian_rendering_domain::Camera {
            frame: Motor3::from_rotation_translation(
                meridian_rendering_domain::look_at_upright(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0)),
                Vec3::ZERO,
            ),
            projection: Projection::perspective(1.0, 64.0 / 48.0, 0.1, 100.0),
        },
        ..Scene3d::default()
    };
    scene.renderables.push(Renderable3d {
        mesh,
        material,
        frame: Motor3::identity(),
        scale: Scale3::UNIT,
        billboard: false,
        casts_shadow: false,
        palette: None,
        morph_weights: Vec::new(),
    });
    scene
}

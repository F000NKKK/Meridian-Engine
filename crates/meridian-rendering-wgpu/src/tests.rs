//! Tests of this module.
#![cfg(feature = "hw-tests")]

use meridian_platform_winit::BackendCapabilities;

use super::*;

/// Every test here needs a real adapter; some CI/sandboxed
/// environments have none. Skip rather than fail in that case.
#[cfg(feature = "hw-tests")]
async fn device_or_skip() -> Option<Device> {
    match Device::new().await {
        Ok(device) => Some(device),
        Err(err) => {
            eprintln!("skipping: no GPU device available ({err})");
            None
        }
    }
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn device_reports_a_nonempty_adapter_name() {
    let Some(device) = device_or_skip().await else {
        return;
    };
    assert_ne!(device.adapter_name(), "");
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn device_reports_gpu_capabilities_matching_adapter_name() {
    let Some(device) = device_or_skip().await else {
        return;
    };
    let gpu = device
        .gpu()
        .expect("a constructed Device always has a real adapter");
    assert_eq!(gpu.device_name, device.adapter_name());
    assert!(device.gpu_available());
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn buffer_write_then_read_round_trips() {
    let Some(device) = device_or_skip().await else {
        return;
    };
    let data: [u8; 8] = [1, 2, 3, 4, 5, 6, 7, 8];
    let buffer = device.create_buffer(data.len(), BufferUsage::Storage);
    device.write_buffer(&buffer, &data);
    let read_back = device.read_buffer(&buffer).await;
    assert_eq!(read_back, data);
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn texture_2d_upload_and_sampler_creation_do_not_panic() {
    let Some(device) = device_or_skip().await else {
        return;
    };
    // 2x2 RGBA8: enough to exercise write_texture's row-layout math
    // without needing a real decoded image asset.
    let pixels: [u8; 16] = [
        255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
    ];
    let texture = device.create_texture_2d(2, 2);
    device.write_texture(&texture, &pixels);
    let _sampler = device.create_sampler();
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn an_offscreen_recorder_reads_back_its_frame_and_times_its_passes() {
    use meridian_rendering_app::{FrameOutcome, FrameRecorder, FrameRequest};
    use meridian_rendering_domain::{MaterialRegistry, MeshRegistry, Scene3d};

    let Some(device) = device_or_skip().await else {
        return;
    };
    let surface = Surface::offscreen(&device, 64, 48);
    let mut recorder = GpuFrameRecorder::new(device, surface, 64, 48);
    let timed = recorder.enable_profiling();
    let (meshes, materials, scene) = (
        MeshRegistry::new(),
        MaterialRegistry::new(),
        Scene3d::default(),
    );
    let outcome = recorder.record_frame(&FrameRequest {
        scene: &scene,
        meshes: &meshes,
        materials: &materials,
        overlays: &[],
        viewports: &[],
        clear_color: [1.0, 0.0, 0.0, 1.0],
    });
    assert_eq!(outcome, FrameOutcome::Presented);
    let pixels = recorder
        .read_pixels()
        .expect("an offscreen frame reads back");
    assert_eq!(pixels.len(), 64 * 48 * 4);
    assert_eq!(recorder.frame_size(), (64, 48));
    assert_eq!(!recorder.pass_timings().is_empty(), timed);
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn a_secondary_viewport_cannot_replace_the_main_camera() {
    use meridian_rendering_app::{FrameOutcome, FrameRecorder, FrameRequest, TextureStore};
    use meridian_rendering_domain::{
        Material, MaterialRegistry, MeshRegistry, Scene3d, Viewport3d,
    };

    let Some(device) = device_or_skip().await else {
        return;
    };
    let surface = Surface::offscreen(&device, 64, 48);
    let mut recorder = GpuFrameRecorder::new(device, surface, 64, 48);
    let mut meshes = MeshRegistry::new();
    let mesh = meshes.register(camera_test_triangle()).unwrap();
    let white = recorder.upload_solid([255; 4]);
    for albedo in [None, Some(white)] {
        let mut materials = MaterialRegistry::new();
        let material = materials.register(Material {
            albedo,
            base_color_factor: [1.0; 4],
            unlit: true,
            ..Material::default()
        });
        let scene = camera_test_scene(mesh, material);
        let viewport = Viewport3d {
            scene: Scene3d::default(),
            rect: [0, 0, 8, 8],
        };
        let mut draw = |viewports: &[Viewport3d]| {
            assert_eq!(
                recorder.record_frame(&FrameRequest {
                    scene: &scene,
                    meshes: &meshes,
                    materials: &materials,
                    overlays: &[],
                    viewports,
                    clear_color: [0.0, 0.0, 0.0, 1.0],
                }),
                FrameOutcome::Presented
            );
            recorder.read_pixels().expect("real pixels")
        };
        let expected = draw(&[]);
        let actual = draw(std::slice::from_ref(&viewport));
        assert_eq!(
            actual, expected,
            "main camera changed with albedo {albedo:?}"
        );
    }
}

#[cfg(feature = "hw-tests")]
fn camera_test_triangle() -> meridian_rendering_domain::MeshSource {
    meridian_rendering_domain::MeshSource {
        positions: vec![[-0.5, -0.5, -2.0], [0.5, -0.5, -2.0], [0.0, 0.5, -2.0]],
        normals: vec![[0.0, 0.0, 1.0]; 3],
        uvs: vec![[0.0; 2]; 3],
        indices: vec![0, 1, 2],
        skin: None,
        morph: None,
    }
}

#[cfg(feature = "hw-tests")]
fn camera_test_scene(
    mesh: meridian_rendering_domain::MeshHandle,
    material: meridian_rendering_domain::MaterialHandle,
) -> meridian_rendering_domain::Scene3d {
    use meridian_gac::{Motor3, Projection, Scale3, Vec3};
    use meridian_rendering_domain::{Camera, Renderable3d, Scene3d, look_at_upright};

    let mut scene = Scene3d {
        camera: Camera {
            frame: Motor3::from_rotation_translation(
                look_at_upright(Vec3::ZERO, Vec3::new(0.0, 0.0, -1.0)),
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

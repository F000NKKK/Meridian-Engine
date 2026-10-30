//! Explicit editor benchmark: a real bound sphere, grid, camera motion and RGBA transfer.

use std::time::Instant;

use meridian_sdk::persistence::{
    ComponentName, ComponentRegistry, NoResolver, Transform, TransformCodec, World,
    register_rendering,
};
use meridian_sdk::{BuiltinAssets, EditorCamera, EditorRenderer, ViewOptions, extract_scene3d};

#[test]
#[ignore = "requires a real adapter; run explicitly to measure editor frame latency"]
fn editor_frame_budget() {
    let mut world = authored_world();
    let mut renderer = EditorRenderer::new(1280, 720).expect("a real GPU adapter is required");
    renderer.bind_references(&mut world, &mut BuiltinAssets::default());
    let adapter = renderer
        .session_mut()
        .render
        .recorder
        .adapter_name()
        .to_owned();
    let mut camera = EditorCamera {
        target: [0.0; 3],
        yaw: 0.6,
        pitch: 0.35,
        distance: 8.0,
        fov_y: 1.0,
    };
    assert_eq!(
        extract_scene3d(&world, camera.camera(1280.0 / 720.0))
            .renderables
            .len(),
        1
    );
    let options = ViewOptions {
        grid: true,
        selection: &[],
    };
    let mut samples = Vec::new();
    for index in 0..130_u16 {
        camera.yaw += 0.002;
        let start = Instant::now();
        let pixels = renderer
            .render_with(&world, &camera, &options)
            .expect("real pixels");
        let mut wire = Vec::with_capacity(8 + pixels.len());
        wire.extend_from_slice(&1280_u32.to_le_bytes());
        wire.extend_from_slice(&720_u32.to_le_bytes());
        wire.extend_from_slice(pixels);
        drop(std::hint::black_box(wire));
        if index >= 10 {
            samples.push(start.elapsed().as_secs_f64() * 1000.0);
        }
    }
    samples.sort_by(f64::total_cmp);
    eprintln!("MERIDIAN_SDK_EDITOR_BUDGET {}", report(&adapter, &samples));
}

fn report(adapter: &str, samples: &[f64]) -> String {
    format!(
        "adapter={adapter:?} size=1280x720 frames={} p50_ms={} p95_ms={} p99_ms={} target_ms=8.333333 target_met_p95={}",
        samples.len(),
        samples[60],
        samples[113],
        samples[118],
        samples[113] <= 1000.0 / 120.0
    )
}

fn authored_world() -> World {
    let mut registry = ComponentRegistry::new();
    registry
        .register_with::<Transform, TransformCodec>(
            ComponentName::new("transform").expect("valid component name"),
        )
        .expect("valid authored scene setup");
    register_rendering(&mut registry).expect("valid authored scene setup");
    let mut world = World::new();
    registry
        .load_scene(
            r#"{format:1,entities:[{id:"sphere",components:{
      transform:{motor:{rotor:[1,0,0,0]}},
      mesh_renderer:{mesh:"@asset:builtin/sphere",material:"@asset:builtin/default"}
    }}]}"#,
            &mut world,
            &mut NoResolver,
        )
        .expect("valid authored scene setup");
    world
}

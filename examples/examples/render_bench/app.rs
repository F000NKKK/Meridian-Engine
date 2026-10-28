//! The benchmark's frame loop, on a headless device.

#![expect(
    clippy::cast_precision_loss,
    reason = "grid indices and frame counts are far below 2^24"
)]

use std::time::Instant;

use meridian_sdk::{Camera, GraphicsSession, Motor3, Projection, Vec3, look_at_upright};

use crate::Options;
use crate::report::{Report, Samples};
use crate::scenes::BenchScene;

/// Renders the scene `options` names and returns what it measured.
pub(crate) fn run(options: &Options, width: u32, height: u32) -> Result<Report, String> {
    let runtime = tokio::runtime::Runtime::new().map_err(|err| err.to_string())?;
    let mut base = runtime
        .block_on(GraphicsSession::headless(width, height))
        .map_err(|err| format!("cannot open a GPU device: {err}"))?;
    let gpu_timestamps = base.render.recorder.enable_profiling();
    let mut scene = BenchScene::build(
        &mut base,
        options.kind,
        options.model.as_deref(),
        options.animation,
    )?;
    let mut report = Report {
        scene: options.kind.name(),
        adapter: base.render.recorder.adapter_name().to_string(),
        size: (width, height),
        frames: options.frames,
        update: Samples::default(),
        render: Samples::default(),
        passes: Vec::new(),
        gpu_timestamps,
    };
    let step = 1.0 / options.fps;
    for frame in 0..options.warmup + options.frames {
        let started = Instant::now();
        let time = (frame + 1) as f32 * step;
        scene.update(step, time);
        point_camera(&mut scene, time, (width, height));
        let updated = Instant::now();
        let pixels = base.render_to_pixels(&scene.scene, [0.45, 0.55, 0.7, 1.0]);
        let rendered = Instant::now();
        if frame >= options.warmup {
            report
                .update
                .push((updated - started).as_secs_f64() * 1000.0);
            report
                .render
                .push((rendered - updated).as_secs_f64() * 1000.0);
            report.add_passes(base.render.recorder.pass_timings());
            if let (Some(dir), Some(pixels)) = (&options.dump, pixels) {
                write_ppm(dir, frame - options.warmup, (width, height), &pixels)?;
            }
        }
    }
    Ok(report)
}

/// An orbit around the scene at 0.4 rad/s.
fn point_camera(scene: &mut BenchScene, time: f32, size: (u32, u32)) {
    let angle = time * 0.4;
    let distance = scene.radius * 2.2;
    let eye = scene.center
        + Vec3::new(
            distance * angle.sin(),
            scene.radius * 0.7,
            distance * angle.cos(),
        );
    #[expect(clippy::cast_precision_loss, reason = "frame sizes are far below 2^24")]
    let aspect = size.0 as f32 / size.1.max(1) as f32;
    scene.scene.camera = Camera {
        frame: Motor3::from_rotation_translation(look_at_upright(eye, scene.center), eye),
        projection: Projection::perspective(1.0, aspect, scene.radius * 0.02, scene.radius * 20.0),
    };
}

/// Writes frame `index` as a binary PPM (no alpha), which `ffmpeg` reads as an
/// image sequence (`-i %05d.ppm`).
fn write_ppm(dir: &str, index: usize, size: (u32, u32), rgba: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|err| format!("cannot create {dir}: {err}"))?;
    let mut bytes = format!("P6\n{} {}\n255\n", size.0, size.1).into_bytes();
    for pixel in rgba.as_chunks::<4>().0 {
        bytes.extend_from_slice(&pixel[..3]);
    }
    let path = format!("{dir}/{index:05}.ppm");
    std::fs::write(&path, bytes).map_err(|err| format!("cannot write {path}: {err}"))
}

//! Loads a glTF model (`.glb`, or `.gltf` with its files beside it) and
//! orbits a camera around it under a sun and a sky: the check that real
//! assets (ADR 030) reach the screen. Pass the model path as the first
//! argument or in `MERIDIAN_MODEL`; an optional second argument is the
//! camera distance in model radii (default 2.6; below 1 the camera is inside
//! the model's bounds, for a scene like Sponza).
//!
//! Design notes: docs/notes/meridian-examples.md (`examples/gltf_viewer`).

mod app;

use app::App;
use meridian_sdk::run_windowed_app;

fn main() {
    meridian_examples::app_main::install_diagnostics("gltf_viewer");
    let path = std::env::args()
        .nth(1)
        .or_else(|| std::env::var("MERIDIAN_MODEL").ok())
        .expect("usage: gltf_viewer <model.glb|model.gltf> (or set MERIDIAN_MODEL)");
    let distance = std::env::args()
        .nth(2)
        .and_then(|text| text.parse().ok())
        .unwrap_or(2.6);
    run_windowed_app(
        "Meridian Engine — glTF Viewer",
        1024,
        768,
        App::new(path, distance),
    )
    .expect("windowed app exited with an error");
}

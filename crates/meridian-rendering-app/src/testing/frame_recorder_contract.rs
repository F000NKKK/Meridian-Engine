//! The behaviour every [`FrameRecorder`](crate::FrameRecorder) must have.

use meridian_rendering_domain::scene::Scene3d;
use meridian_rendering_domain::{EnvironmentMap, MaterialRegistry, MeshRegistry, Overlay2d, Sky};

use crate::{FrameOutcome, FrameRecorder, FrameRequest};

/// Asserts the frame recorder contract against fresh recorders from `make`: an
/// empty scene with an empty overlay is presented; a frame after a resize is presented; a frame
/// is presented with an environment set and again with it cleared; and a
/// recorder whose target `lose_target` took away reports
/// [`FrameOutcome::TargetUnavailable`] until it is resized.
///
/// # Panics
///
/// Panics when `R` breaks the contract.
pub fn run_frame_recorder_contract<R: FrameRecorder>(
    make: impl Fn() -> R,
    lose_target: impl Fn(&mut R),
) {
    let scene = Scene3d::default();
    let meshes = MeshRegistry::new();
    let materials = MaterialRegistry::new();
    let overlays = [Overlay2d::default()];
    let request = FrameRequest {
        scene: &scene,
        meshes: &meshes,
        materials: &materials,
        overlays: &overlays,
        viewports: &[],
        clear_color: [0.1, 0.2, 0.3, 1.0],
    };

    let mut recorder = make();
    assert_eq!(recorder.record_frame(&request), FrameOutcome::Presented);
    recorder.resize(128, 96);
    assert_eq!(recorder.record_frame(&request), FrameOutcome::Presented);

    let environment = EnvironmentMap::from_sky(&Sky::default()).expect("the default sky builds");
    recorder.set_environment(Some(&environment));
    assert_eq!(recorder.record_frame(&request), FrameOutcome::Presented);
    recorder.set_environment(None);
    assert_eq!(recorder.record_frame(&request), FrameOutcome::Presented);

    lose_target(&mut recorder);
    assert_eq!(
        recorder.record_frame(&request),
        FrameOutcome::TargetUnavailable
    );
    recorder.resize(128, 96);
    assert_eq!(
        recorder.record_frame(&request),
        FrameOutcome::Presented,
        "a resize gives the target back"
    );
}

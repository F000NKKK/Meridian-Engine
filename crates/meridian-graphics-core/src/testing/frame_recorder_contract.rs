//! The behaviour every [`FrameRecorder`](crate::FrameRecorder) must have.

use crate::scene::Scene3d;
use crate::{FrameOutcome, FrameRecorder, FrameRequest, MaterialRegistry, MeshRegistry};

/// Asserts the frame recorder contract against fresh recorders from `make`: an
/// empty scene is presented; a frame after a resize is presented; and a
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
    let request = FrameRequest {
        scene: &scene,
        meshes: &meshes,
        materials: &materials,
        clear_color: [0.1, 0.2, 0.3, 1.0],
    };

    let mut recorder = make();
    assert_eq!(recorder.record_frame(&request), FrameOutcome::Presented);
    recorder.resize(128, 96);
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

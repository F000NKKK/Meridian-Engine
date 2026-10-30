//! A [`FrameRecorder`](crate::FrameRecorder) that draws nothing.

use meridian_rendering_domain::EnvironmentMap;

use crate::{FrameOutcome, FrameRecorder, FrameRequest};

/// Counts the frames it is asked for and reports what it was told to.
#[derive(Debug)]
pub struct FakeFrameRecorder {
    size: [u32; 2],
    frames: usize,
    overlays: usize,
    available: bool,
    environment: bool,
}

impl FakeFrameRecorder {
    /// A recorder with a `width` by `height` target that presents every frame.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            size: [width, height],
            frames: 0,
            overlays: 0,
            available: true,
            environment: false,
        }
    }

    /// Makes the target report itself lost until the next resize, as a
    /// swapchain does.
    pub fn lose_target(&mut self) {
        self.available = false;
    }

    /// Frames presented so far.
    #[must_use]
    pub fn presented(&self) -> usize {
        self.frames
    }

    /// Overlays handed to presented frames so far.
    #[must_use]
    pub fn overlays_drawn(&self) -> usize {
        self.overlays
    }

    /// Whether an environment is set.
    #[must_use]
    pub fn has_environment(&self) -> bool {
        self.environment
    }

    /// The current `[width, height]`.
    #[must_use]
    pub fn size(&self) -> [u32; 2] {
        self.size
    }
}

impl FrameRecorder for FakeFrameRecorder {
    fn record_frame(&mut self, request: &FrameRequest<'_>) -> FrameOutcome {
        if !self.available {
            return FrameOutcome::TargetUnavailable;
        }
        self.frames += 1;
        self.overlays += request.overlays.len();
        FrameOutcome::Presented
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.size = [width, height];
        self.available = true;
    }

    fn set_environment(&mut self, environment: Option<&EnvironmentMap>) {
        self.environment = environment.is_some();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::run_frame_recorder_contract;

    #[test]
    fn remembers_whether_an_environment_is_set() {
        let mut recorder = FakeFrameRecorder::new(8, 8);
        let map = EnvironmentMap::from_sky(&meridian_rendering_domain::Sky::default()).unwrap();
        recorder.set_environment(Some(&map));
        assert!(recorder.has_environment());
        recorder.set_environment(None);
        assert!(!recorder.has_environment());
    }

    #[test]
    fn passes_the_frame_recorder_contract() {
        run_frame_recorder_contract(
            || FakeFrameRecorder::new(64, 64),
            |recorder| {
                recorder.lose_target();
            },
        );
    }

    #[test]
    fn counts_presented_frames() {
        let mut recorder = FakeFrameRecorder::new(8, 8);
        let scene = meridian_rendering_domain::scene::Scene3d::default();
        let request = FrameRequest {
            scene: &scene,
            meshes: &meridian_rendering_domain::MeshRegistry::new(),
            materials: &meridian_rendering_domain::MaterialRegistry::new(),
            overlays: &[],
            viewports: &[],
            clear_color: [0.0; 4],
        };
        assert_eq!(recorder.record_frame(&request), FrameOutcome::Presented);
        assert_eq!(recorder.presented(), 1);
        assert_eq!(recorder.size(), [8, 8]);
    }
}

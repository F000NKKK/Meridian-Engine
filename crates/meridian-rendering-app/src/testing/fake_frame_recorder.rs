//! A [`FrameRecorder`](crate::FrameRecorder) that draws nothing.

use crate::{FrameOutcome, FrameRecorder, FrameRequest};

/// Counts the frames it is asked for and reports what it was told to.
#[derive(Debug)]
pub struct FakeFrameRecorder {
    size: [u32; 2],
    frames: usize,
    available: bool,
}

impl FakeFrameRecorder {
    /// A recorder with a `width` by `height` target that presents every frame.
    #[must_use]
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            size: [width, height],
            frames: 0,
            available: true,
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

    /// The current `[width, height]`.
    #[must_use]
    pub fn size(&self) -> [u32; 2] {
        self.size
    }
}

impl FrameRecorder for FakeFrameRecorder {
    fn record_frame(&mut self, _request: &FrameRequest<'_>) -> FrameOutcome {
        if !self.available {
            return FrameOutcome::TargetUnavailable;
        }
        self.frames += 1;
        FrameOutcome::Presented
    }

    fn resize(&mut self, width: u32, height: u32) {
        self.size = [width, height];
        self.available = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::run_frame_recorder_contract;

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
            clear_color: [0.0; 4],
        };
        assert_eq!(recorder.record_frame(&request), FrameOutcome::Presented);
        assert_eq!(recorder.presented(), 1);
        assert_eq!(recorder.size(), [8, 8]);
    }
}

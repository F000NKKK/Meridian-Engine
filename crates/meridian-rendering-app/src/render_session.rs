//! What an application renders with: a recorder plus the registries its scenes resolve against.

use meridian_graphics_core::scene::Scene3d;
use meridian_graphics_core::{MaterialRegistry, MeshRegistry};

use crate::{FrameOutcome, FrameRecorder, FrameRequest};

/// A [`FrameRecorder`] bundled with the mesh and material registries a
/// [`Scene3d`]'s handles resolve against, and the one rule of presenting a
/// frame: a lost target is resized to the window's size and the frame skipped
/// (the caller asks again next redraw).
#[derive(Debug)]
pub struct RenderSession<R> {
    /// Draws the frames; the adapter that owns the device and the swapchain.
    pub recorder: R,
    /// Meshes the scene's `MeshHandle`s resolve against.
    pub meshes: MeshRegistry,
    /// Materials the scene's `MaterialHandle`s resolve against.
    pub materials: MaterialRegistry,
}

impl<R: FrameRecorder> RenderSession<R> {
    /// A session drawing with `recorder`, with empty registries to fill.
    #[must_use]
    pub fn new(recorder: R) -> Self {
        Self {
            recorder,
            meshes: MeshRegistry::new(),
            materials: MaterialRegistry::new(),
        }
    }

    /// Draws `scene` and presents it. When the target is lost it is resized
    /// to `target_size` (`(width, height)` of the window) and the frame is
    /// skipped; the outcome says which happened.
    pub fn render(
        &mut self,
        scene: &Scene3d,
        clear_color: [f64; 4],
        target_size: (u32, u32),
    ) -> FrameOutcome {
        let request = FrameRequest {
            scene,
            meshes: &self.meshes,
            materials: &self.materials,
            clear_color,
        };
        let outcome = self.recorder.record_frame(&request);
        if outcome == FrameOutcome::TargetUnavailable {
            self.recorder.resize(target_size.0, target_size.1);
        }
        outcome
    }

    /// Rebuilds every size-dependent piece after a window resize.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.recorder.resize(width, height);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::FakeFrameRecorder;

    #[test]
    fn presents_while_the_target_is_available() {
        let mut session = RenderSession::new(FakeFrameRecorder::new(8, 8));
        let outcome = session.render(&Scene3d::default(), [0.0; 4], (8, 8));
        assert_eq!(outcome, FrameOutcome::Presented);
        assert_eq!(session.recorder.presented(), 1);
    }

    #[test]
    fn a_lost_target_is_resized_to_the_window_and_the_frame_skipped() {
        let mut session = RenderSession::new(FakeFrameRecorder::new(8, 8));
        session.recorder.lose_target();

        let outcome = session.render(&Scene3d::default(), [0.0; 4], (640, 480));
        assert_eq!(outcome, FrameOutcome::TargetUnavailable);
        assert_eq!(session.recorder.size(), [640, 480]);

        let outcome = session.render(&Scene3d::default(), [0.0; 4], (640, 480));
        assert_eq!(outcome, FrameOutcome::Presented);
    }
}

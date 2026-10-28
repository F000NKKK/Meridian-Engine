//! The port that draws a scene to a target.

use crate::{FrameOutcome, FrameRequest};

/// Draws frames to a render target: the rendering context's output port.
///
/// An implementation owns the device, the swapchain and every pipeline.
/// `rendering-wgpu` implements it over `wgpu`;
/// [`testing::FakeFrameRecorder`](crate::testing::FakeFrameRecorder) is the fake
/// and [`testing::run_frame_recorder_contract`](crate::testing::run_frame_recorder_contract)
/// the suite an implementation passes. A windowed adapter needs a real window,
/// so its contract run is a manual `hw-tests` job.
pub trait FrameRecorder {
    /// Records `request` into the target and presents it.
    fn record_frame(&mut self, request: &FrameRequest<'_>) -> FrameOutcome;

    /// Resizes the target and everything sized to it.
    fn resize(&mut self, width: u32, height: u32);
}

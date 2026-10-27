//! What happened to a requested frame.

/// The result of [`FrameRecorder::record_frame`](crate::FrameRecorder::record_frame).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FrameOutcome {
    /// The frame was recorded and presented.
    Presented,
    /// The target had no frame to draw into (a lost or outdated swapchain); the
    /// recorder skipped the frame and the caller should resize and ask again.
    TargetUnavailable,
}

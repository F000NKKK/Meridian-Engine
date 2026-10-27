//! Why acquiring a surface frame failed.

/// Why [`Surface::acquire_frame`](crate::Surface::acquire_frame) didn't return a frame this call. Every
/// variant is a real, expected transient condition (see
/// `wgpu::CurrentSurfaceTexture`'s own variant docs) — a caller should
/// skip the frame and try again next time, not treat this as fatal,
/// except [`AcquireFrameError::Lost`] (needs the whole surface, not just
/// a frame, recreated) which is rare enough in practice not to warrant
/// its own recovery path here yet.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AcquireFrameError {
    Timeout,
    Occluded,
    Outdated,
    Lost,
    Validation,
}

impl std::fmt::Display for AcquireFrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AcquireFrameError::Timeout => write!(f, "timed out acquiring the next swapchain frame"),
            AcquireFrameError::Occluded => write!(f, "surface is occluded (minimized or hidden)"),
            AcquireFrameError::Outdated => {
                write!(f, "surface configuration is outdated, call Surface::resize")
            }
            AcquireFrameError::Lost => write!(f, "surface was lost and needs to be recreated"),
            AcquireFrameError::Validation => {
                write!(f, "validation error acquiring the next swapchain frame")
            }
        }
    }
}

impl std::error::Error for AcquireFrameError {}

impl meridian_foundation::EngineError for AcquireFrameError {}

//! The default way an application builds its runtime.

use std::sync::Arc;

use meridian_platform_winit::CpuCapabilities;
use meridian_runtime_app::Runtime;
use meridian_runtime_threads::TaskExecutor;

/// A [`Runtime`] whose stages run on a [`TaskExecutor`] sized to the detected
/// CPU thread count: the composition root's choice of adapters, so the
/// runtime crate itself names none.
#[must_use]
pub fn parallel_runtime() -> Runtime {
    Runtime::new(Arc::new(TaskExecutor::new(
        CpuCapabilities::detect().threads,
    )))
}

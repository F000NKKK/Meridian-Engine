//! The default way an application builds its runtime.

use std::sync::Arc;

use meridian_engine_core::Runtime;
use meridian_platform_core::CpuCapabilities;
use meridian_task_core::TaskExecutor;

/// A [`Runtime`] whose stages run on a [`TaskExecutor`] sized to the detected
/// CPU thread count: the composition root's choice of adapters, so the
/// runtime crate itself names none.
#[must_use]
pub fn parallel_runtime() -> Runtime {
    Runtime::new(Arc::new(TaskExecutor::new(
        CpuCapabilities::detect().threads,
    )))
}

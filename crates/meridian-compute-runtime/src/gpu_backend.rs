//! The port for a GPU compute backend.

use std::any::Any;

/// A GPU compute backend: the port a [`ComputeContext`](crate::ComputeContext)
/// holds as its GPU half.
///
/// The application layer only needs to know a GPU exists and what it is called.
/// Kernel adapters (`meridian-gac-compute`, `meridian-physics-compute`) know the
/// concrete adapter type and reach it with
/// [`ComputeContext::gpu`](crate::ComputeContext::gpu). The fake is
/// [`testing::FakeGpu`](crate::testing::FakeGpu) and the suite every
/// implementation passes is
/// [`testing::run_gpu_contract`](crate::testing::run_gpu_contract).
pub trait GpuBackend: std::fmt::Debug + Send + Sync + 'static {
    /// The adapter's human-readable name (never empty).
    fn device_name(&self) -> String;

    /// This backend as `Any`: the way a kernel adapter downcasts to its own
    /// concrete type.
    fn as_any(&self) -> &dyn Any;
}

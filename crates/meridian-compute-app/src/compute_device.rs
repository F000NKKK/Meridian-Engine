//! The port for the CPU side of a compute backend.

use std::any::Any;
use std::ops::Range;

use crate::{ComputeBuffer, ComputeSupport};

/// A compute backend that runs index ranges on the CPU: the port
/// [`ComputeContext`](crate::ComputeContext) dispatches through.
///
/// Mechanism, not policy: [`dispatch_ranges`](Self::dispatch_ranges) always runs the
/// work as the backend sees fit; the sequential cutoff is the context's. Adapters
/// implement it (`meridian-compute-wgpu` with OS threads);
/// `testing::SequentialDevice` is the fake and
/// `testing::run_device_contract` the suite
/// every implementation passes.
pub trait ComputeDevice: std::fmt::Debug + Send + Sync + 'static {
    /// What this backend reports about itself.
    fn support(&self) -> ComputeSupport;

    /// A zero-filled buffer of `byte_len` bytes.
    fn allocate_buffer(&self, byte_len: usize) -> ComputeBuffer {
        ComputeBuffer::zeroed(byte_len)
    }

    /// Splits `0..count` into disjoint ranges that together cover it exactly
    /// once and calls `work` once per range, possibly on several threads.
    /// Returns when every range is done. `count == 0` calls nothing.
    fn dispatch_ranges(&self, count: usize, work: &(dyn Fn(Range<usize>) + Sync));

    /// This device as `Any`, so a kernel adapter that knows the concrete type
    /// can reach it.
    fn as_any(&self) -> &dyn Any;
}

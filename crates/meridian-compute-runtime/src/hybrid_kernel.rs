//! A kernel with both a CPU and a GPU implementation.

use std::future::Future;
use std::ops::Range;

use crate::ComputeContext;

/// A dispatchable unit of compute work with *both* a CPU and a GPU
/// implementation, letting [`ComputeContext::run_hybrid`](crate::ComputeContext::run_hybrid) split one
/// logical dispatch across both backends at once — see the module doc
/// for why this is a separate trait from [`ComputeKernel`](crate::ComputeKernel) (a plain
/// `ComputeKernel` might only ever have a CPU implementation, and
/// nothing about its `dispatch` signature could express "also, here's
/// the GPU half").
pub trait HybridKernel {
    /// Runs this kernel's operation for every index in `range`, on the
    /// CPU, blocking the calling thread until done. Runs on a
    /// `tokio::task::spawn_blocking` thread when called via
    /// `run_hybrid`, so it's free to do real CPU-bound work without
    /// stalling an async runtime's worker threads. If the kernel wants
    /// its own internal parallelism across `range` (its own
    /// `std::thread::scope`, for instance), that's the kernel's own
    /// responsibility — this method doesn't receive a `ComputeContext`
    /// precisely so it can be moved into a `'static` task by itself.
    fn run_cpu(&self, range: Range<usize>);

    /// Runs this kernel's operation for every index in `range`, on the
    /// GPU, via `context`'s GPU backend (`context.gpu()` is guaranteed
    /// `Some` whenever `run_hybrid` calls this). A real `async fn`:
    /// reading results back waits on in-flight GPU work — see
    /// [ADR 009](../../../docs/adr/009-async-io-via-tokio.md).
    fn run_gpu(&self, context: &ComputeContext, range: Range<usize>) -> impl Future<Output = ()>;
}

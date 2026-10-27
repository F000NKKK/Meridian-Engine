//! The compute device and its dispatch policy.

use std::sync::Arc;

use meridian_compute_driver::{
    ComputeBuffer, ComputeCapabilities, ComputeDevice, GpuComputeDevice, GpuComputeError,
};

use crate::{BackendSplit, HybridKernel};

/// Everything a `ComputeKernel` dispatch needs: bound buffers, the active
/// `compute-driver` backend, synchronization state. Consumers reach
/// `compute-driver` only through this type — see
/// docs/dependency-rules.md rule 5.
#[derive(Debug, Clone)]
pub struct ComputeContext {
    device: ComputeDevice,
    gpu: Option<GpuComputeDevice>,
    parallel_threshold: usize,
}

impl Default for ComputeContext {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputeContext {
    pub fn new() -> Self {
        Self {
            device: ComputeDevice::new(),
            gpu: None,
            parallel_threshold: 1024,
        }
    }

    pub(crate) fn with_parallel_threshold(mut self, threshold: usize) -> Self {
        self.parallel_threshold = threshold;
        self
    }

    /// Adds `compute-driver`'s real GPU backend to this context — see the
    /// module doc. A real `async fn` (GPU device acquisition is genuine
    /// I/O); returns the `wgpu`/driver error as-is if no adapter is
    /// available rather than silently falling back to CPU-only, so a
    /// caller that specifically wants GPU dispatch finds out immediately.
    pub async fn with_gpu(mut self) -> Result<Self, GpuComputeError> {
        self.gpu = Some(GpuComputeDevice::new().await?);
        Ok(self)
    }

    /// The GPU backend, if [`ComputeContext::with_gpu`] added one — what
    /// a [`ComputeKernel`](crate::ComputeKernel) reaches for to dispatch on the GPU specifically
    /// (see the module doc).
    pub fn gpu(&self) -> Option<&GpuComputeDevice> {
        self.gpu.as_ref()
    }

    pub fn capabilities(&self) -> ComputeCapabilities {
        match &self.gpu {
            Some(gpu) => gpu.capabilities(),
            None => self.device.capabilities(),
        }
    }

    pub fn allocate_buffer(&self, byte_len: usize) -> ComputeBuffer {
        self.device.allocate_buffer(byte_len)
    }

    /// Runs `work(i)` for every `i` in `0..count`. See the module doc for
    /// the sequential/parallel cutoff policy.
    pub fn parallel_for(&self, count: usize, work: impl Fn(usize) + Sync) {
        if count == 0 {
            return;
        }
        if count < self.parallel_threshold {
            for i in 0..count {
                work(i);
            }
            return;
        }
        self.device.dispatch_parallel(count, work);
    }

    /// Splits `count` work items between `kernel`'s CPU and GPU
    /// implementations per `split`, running both halves concurrently —
    /// see the module doc for why this needs `kernel: Arc<K>` rather than
    /// `&K` (the CPU half runs on a `tokio::task::spawn_blocking` thread,
    /// which requires `'static`) and for the mechanism in general. A
    /// no-op if `count == 0`. Falls back to
    /// `CpuOnly` behavior regardless of `split` if this context has no
    /// GPU backend (see [`ComputeContext::with_gpu`]) — a caller that
    /// wants a hybrid dispatch to *require* GPU should check
    /// [`ComputeContext::gpu`] itself first.
    pub async fn run_hybrid<K>(&self, kernel: Arc<K>, count: usize, split: BackendSplit)
    where
        K: HybridKernel + Send + Sync + 'static,
    {
        if count == 0 {
            return;
        }
        let gpu_count = split.gpu_item_count(count, self.gpu.is_some());
        let cpu_count = count - gpu_count;

        if gpu_count == 0 {
            tokio::task::spawn_blocking(move || kernel.run_cpu(0..cpu_count))
                .await
                .expect("HybridKernel::run_cpu task panicked");
            return;
        }
        if cpu_count == 0 {
            kernel.run_gpu(self, 0..gpu_count).await;
            return;
        }

        // Real concurrency: `spawn_blocking` hands the CPU half to its
        // own OS thread immediately, so it's already running while this
        // task goes on to dispatch and await the GPU half — not
        // sequential CPU-then-GPU.
        let cpu_range = 0..cpu_count;
        let gpu_range = cpu_count..count;
        let cpu_kernel = kernel.clone();
        let cpu_task = tokio::task::spawn_blocking(move || cpu_kernel.run_cpu(cpu_range));
        kernel.run_gpu(self, gpu_range).await;
        cpu_task.await.expect("HybridKernel::run_cpu task panicked");
    }
}

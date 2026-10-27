//! The compute device and its dispatch policy.

use std::sync::Arc;

use crate::{BackendSplit, ComputeBuffer, ComputeDevice, ComputeSupport, GpuBackend, HybridKernel};

/// Everything a `ComputeKernel` dispatch needs: the CPU [`ComputeDevice`], an
/// optional [`GpuBackend`], and the sequential/parallel cutoff. The devices are
/// ports; the composition root supplies the adapters (rule 5 of
/// docs/dependency-rules.md: consumers reach compute only through this type).
#[derive(Debug, Clone)]
pub struct ComputeContext {
    device: Arc<dyn ComputeDevice>,
    gpu: Option<Arc<dyn GpuBackend>>,
    parallel_threshold: usize,
}

impl ComputeContext {
    /// A CPU-only context over `device`.
    #[must_use]
    pub fn new(device: Arc<dyn ComputeDevice>) -> Self {
        Self {
            device,
            gpu: None,
            parallel_threshold: 1024,
        }
    }

    pub(crate) fn with_parallel_threshold(mut self, threshold: usize) -> Self {
        self.parallel_threshold = threshold;
        self
    }

    /// Adds a GPU backend to this context. Acquiring the device is the
    /// adapter's I/O (`async`); the context only holds the result. A caller
    /// that wants GPU dispatch and has no adapter finds out when it acquires one,
    /// not through a silent CPU fallback here.
    #[must_use]
    pub fn with_gpu(mut self, gpu: Arc<dyn GpuBackend>) -> Self {
        self.gpu = Some(gpu);
        self
    }

    /// Whether a GPU backend was added.
    #[must_use]
    pub fn has_gpu(&self) -> bool {
        self.gpu.is_some()
    }

    /// The GPU backend as its concrete adapter type `T`, if one was added and
    /// it is a `T`: what a kernel adapter reaches for to dispatch on the GPU.
    #[must_use]
    pub fn gpu<T: GpuBackend>(&self) -> Option<&T> {
        self.gpu.as_deref()?.as_any().downcast_ref()
    }

    /// What the backends report: the CPU side's threads and the GPU's name.
    #[must_use]
    pub fn support(&self) -> ComputeSupport {
        ComputeSupport {
            gpu_device_name: self.gpu.as_ref().map(|gpu| gpu.device_name()),
            ..self.device.support()
        }
    }

    /// A zero-filled CPU buffer from the device.
    #[must_use]
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
        self.device
            .dispatch_ranges(count, &|range| range.for_each(&work));
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
        let gpu_count = split.gpu_item_count(count, self.has_gpu());
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

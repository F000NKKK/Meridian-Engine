//! Compute dispatch runtime: device/context, buffers, and the `ComputeKernel` interface domain crates implement against. Knows nothing about physics, GAC, particles or any other algorithm — see docs/dependency-rules.md rule 5.
//!
//! [`ComputeContext::parallel_for`] is the one place `compute-driver` gets
//! reached (rule 5): below `parallel_threshold` it runs sequentially on
//! the calling thread, at or above it delegates to
//! `compute-driver::ComputeDevice::dispatch_parallel`. That threshold is
//! configurable state (via [`ComputeScheduler`]), not a hardcoded
//! constant domain crates read — the point being that "how many items
//! justify parallelizing" is a policy call, tunable per workload, not a
//! fact about the algorithm.
//!
//! `task-core` is a declared dependency (see docs/threading-model.md) but
//! not yet used here: `JobGraph::add_job` requires `'static` closures,
//! and `parallel_for`'s `work` is borrowed from the caller's stack frame
//! — a real signature mismatch, not an oversight. Reusing `task-core` for
//! this needs a scoped/borrowing job API that doesn't exist yet, so this
//! uses `compute-driver`'s own `std::thread::scope`-based dispatch
//! instead of forcing an awkward fit.
//!
//! [`ComputeContext::with_gpu`] adds `compute-driver`'s real GPU backend
//! (`GpuComputeDevice`, built on `wgpu` via `meridian_gpu_driver` — see
//! that crate's module doc) alongside the CPU one; [`ComputeContext::gpu`]
//! is what a [`ComputeKernel`] reaches for when it wants to dispatch on
//! the GPU specifically (this crate doesn't pick a backend on a kernel's
//! behalf — that decision belongs to whoever composes the `ComputeContext`
//! in the first place, e.g. `gac-compute`'s `Fixed` kernels choosing GPU
//! dispatch explicitly). GPU device acquisition is a genuine I/O
//! operation, so `with_gpu` is a real `async fn` — see
//! [ADR 009](../../../docs/adr/009-async-io-via-tokio.md).
//!
//! [`HybridKernel`]/[`ComputeContext::run_hybrid`] are the actual "split
//! one dispatch across both backends at once" mechanism — plain
//! [`ComputeKernel`]/[`ComputeContext::parallel_for`] can only ever run
//! on the CPU (an arbitrary `impl Fn(usize)` closure has no WGSL
//! equivalent a compiler can derive for you, so there is no such thing as
//! an automatic "run this same code on either backend" switch — a kernel
//! author has to write both implementations by hand, the same way
//! `gac-compute::fixed_wgsl` hand-writes its WGSL alongside
//! `meridian_numeric_core::Fixed`'s existing Rust). A kernel that
//! *does* provide both implements [`HybridKernel`]
//! (`run_cpu`/`run_gpu`, one method per backend, each covering an index
//! range rather than the whole dispatch), and [`ComputeContext::run_hybrid`]
//! splits `count` items between them per a [`BackendSplit`] policy
//! (`Ratio(0.5)` for an even split, among others) and runs both halves
//! *concurrently*, not sequentially: the CPU half runs on
//! `tokio::task::spawn_blocking`'s dedicated thread pool (needing
//! `K: Send + Sync + 'static`, hence the `Arc<K>` in the signature) while
//! this task directly awaits the GPU half's dispatch+readback — real
//! overlap, not "CPU work, then wait for GPU work."

use std::future::Future;
use std::ops::Range;
use std::sync::Arc;

use meridian_compute_driver::{
    ComputeBuffer, ComputeCapabilities, ComputeDevice, GpuComputeDevice, GpuComputeError,
};

/// Device-visible dispatch dimensions for a `ComputeKernel` invocation.
#[derive(Debug, Clone, Copy, Default)]
pub struct DispatchSize {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl DispatchSize {
    /// A 1D dispatch of `count` work items (`y = z = 1`) — what every
    /// current `ComputeKernel` (batch `Motor3` work) uses.
    pub fn linear(count: u32) -> Self {
        Self {
            x: count,
            y: 1,
            z: 1,
        }
    }

    pub fn total(&self) -> usize {
        self.x as usize * self.y as usize * self.z as usize
    }
}

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

    fn with_parallel_threshold(mut self, threshold: usize) -> Self {
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
    /// a [`ComputeKernel`] reaches for to dispatch on the GPU specifically
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
    /// no-op (`Ok`, does nothing) if `count == 0`. Falls back to
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

/// How [`ComputeContext::run_hybrid`] splits `count` work items between
/// CPU and GPU.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum BackendSplit {
    /// Every item on the CPU.
    CpuOnly,
    /// Every item on the GPU.
    GpuOnly,
    /// `0.0..=1.0`: that fraction of items on the GPU, the rest on the
    /// CPU. `Ratio(0.5)` is an even 50/50 split. Out-of-range values are
    /// clamped, not a logic error — a caller computing a ratio
    /// dynamically (e.g. from a benchmark) shouldn't have to re-clamp it
    /// themselves.
    Ratio(f32),
}

impl BackendSplit {
    fn gpu_item_count(self, count: usize, gpu_available: bool) -> usize {
        if !gpu_available {
            return 0;
        }
        match self {
            BackendSplit::CpuOnly => 0,
            BackendSplit::GpuOnly => count,
            BackendSplit::Ratio(fraction) => {
                let fraction = fraction.clamp(0.0, 1.0);
                ((count as f32) * fraction).round() as usize
            }
        }
    }
}

/// A dispatchable unit of compute work with *both* a CPU and a GPU
/// implementation, letting [`ComputeContext::run_hybrid`] split one
/// logical dispatch across both backends at once — see the module doc
/// for why this is a separate trait from [`ComputeKernel`] (a plain
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

/// A dispatchable unit of compute work. Domain crates (`gac-compute`, and
/// future `particle-compute`/`physics-compute`/`ai-compute`) implement this
/// for their own kernel types; `compute-runtime` knows nothing about what a
/// kernel computes, only how it gets dispatched and on which backend.
pub trait ComputeKernel {
    fn dispatch(&self, context: &ComputeContext, size: DispatchSize);
}

/// The entry point consumers use instead of calling `ComputeKernel::dispatch`
/// directly: owns the [`ComputeContext`] (and its sequential/parallel
/// threshold policy) a kernel dispatch runs against.
#[derive(Debug, Clone)]
pub struct ComputeScheduler {
    context: ComputeContext,
}

impl Default for ComputeScheduler {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputeScheduler {
    pub fn new() -> Self {
        Self {
            context: ComputeContext::new(),
        }
    }

    /// Below `threshold` work items, a dispatch runs sequentially on the
    /// calling thread rather than fanning out across `compute-driver`'s
    /// worker threads.
    pub fn with_parallel_threshold(threshold: usize) -> Self {
        Self {
            context: ComputeContext::new().with_parallel_threshold(threshold),
        }
    }

    pub fn context(&self) -> &ComputeContext {
        &self.context
    }

    pub fn run<K: ComputeKernel>(&self, kernel: &K, size: DispatchSize) {
        kernel.dispatch(&self.context, size);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    struct CountingKernel<'a> {
        counter: &'a AtomicUsize,
    }

    impl ComputeKernel for CountingKernel<'_> {
        fn dispatch(&self, context: &ComputeContext, size: DispatchSize) {
            context.parallel_for(size.total(), |_| {
                self.counter.fetch_add(1, Ordering::SeqCst);
            });
        }
    }

    #[test]
    fn dispatch_size_linear_and_total() {
        let size = DispatchSize::linear(42);
        assert_eq!(size.x, 42);
        assert_eq!(size.y, 1);
        assert_eq!(size.z, 1);
        assert_eq!(size.total(), 42);
    }

    #[test]
    fn context_capabilities_report_no_gpu_by_default() {
        assert!(ComputeContext::new().capabilities().gpu.is_none());
    }

    /// Needs a real adapter; some CI/sandboxed environments have none —
    /// skip rather than fail, matching every other GPU-touching test in
    /// this workspace.
    #[tokio::test]
    async fn context_with_gpu_reports_a_real_device_name() {
        let ctx = match ComputeContext::new().with_gpu().await {
            Ok(ctx) => ctx,
            Err(err) => {
                eprintln!("skipping: no GPU device available ({err})");
                return;
            }
        };
        assert!(ctx.gpu().is_some());
        let gpu = ctx
            .capabilities()
            .gpu
            .expect("with_gpu succeeded, so capabilities().gpu must be Some");
        assert!(!gpu.device_name.is_empty());
    }

    #[test]
    fn parallel_for_below_threshold_still_visits_every_index() {
        let ctx = ComputeContext::new().with_parallel_threshold(1_000_000); // force sequential path
        let counter = AtomicUsize::new(0);
        ctx.parallel_for(10, |_| {
            counter.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(counter.load(Ordering::SeqCst), 10);
    }

    #[test]
    fn parallel_for_above_threshold_still_visits_every_index() {
        let ctx = ComputeContext::new().with_parallel_threshold(1); // force parallel path
        let counter = AtomicUsize::new(0);
        ctx.parallel_for(5_000, |_| {
            counter.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(counter.load(Ordering::SeqCst), 5_000);
    }

    #[test]
    fn scheduler_run_dispatches_kernel() {
        let counter = AtomicUsize::new(0);
        let kernel = CountingKernel { counter: &counter };
        let scheduler = ComputeScheduler::with_parallel_threshold(1);
        scheduler.run(&kernel, DispatchSize::linear(2_000));
        assert_eq!(counter.load(Ordering::SeqCst), 2_000);
    }

    #[test]
    fn backend_split_gpu_item_count() {
        assert_eq!(BackendSplit::CpuOnly.gpu_item_count(100, true), 0);
        assert_eq!(BackendSplit::GpuOnly.gpu_item_count(100, true), 100);
        assert_eq!(
            BackendSplit::GpuOnly.gpu_item_count(100, false),
            0,
            "no GPU backend available must fall back to zero GPU items regardless of policy"
        );
        assert_eq!(BackendSplit::Ratio(0.5).gpu_item_count(100, true), 50);
        assert_eq!(
            BackendSplit::Ratio(2.0).gpu_item_count(100, true),
            100,
            "out-of-range ratios must clamp, not panic or produce a bogus count"
        );
        assert_eq!(BackendSplit::Ratio(-1.0).gpu_item_count(100, true), 0);
    }

    /// A `HybridKernel` whose `run_gpu` always panics — used to prove
    /// [`ComputeContext::run_hybrid`] never calls it when the context has
    /// no GPU backend (a real assertion failure if the fallback logic is
    /// wrong, not just an absence of evidence).
    struct PanicsOnGpuKernel {
        cpu_visited: std::sync::Mutex<Vec<usize>>,
    }

    impl HybridKernel for PanicsOnGpuKernel {
        fn run_cpu(&self, range: Range<usize>) {
            self.cpu_visited.lock().unwrap().extend(range);
        }

        async fn run_gpu(&self, _context: &ComputeContext, _range: Range<usize>) {
            panic!("run_gpu must never be called when the context has no GPU backend");
        }
    }

    #[tokio::test]
    async fn run_hybrid_without_gpu_backend_falls_back_to_cpu_only() {
        let context = ComputeContext::new(); // no with_gpu()
        let kernel = Arc::new(PanicsOnGpuKernel {
            cpu_visited: std::sync::Mutex::new(Vec::new()),
        });
        context
            .run_hybrid(kernel.clone(), 50, BackendSplit::Ratio(0.5))
            .await;
        let mut visited = kernel.cpu_visited.lock().unwrap().clone();
        visited.sort_unstable();
        assert_eq!(visited, (0..50).collect::<Vec<_>>());
    }

    /// A real doubling kernel: `run_cpu` doubles in Rust, `run_gpu`
    /// dispatches a real WGSL compute shader that does the same thing —
    /// both write into a shared output buffer by index, so this proves
    /// `run_hybrid` both runs each half correctly *and* merges their
    /// results into the right positions, not just that each half runs in
    /// isolation.
    struct DoublingKernel {
        input: Vec<u32>,
        output: std::sync::Mutex<Vec<u32>>,
        pipeline: meridian_gpu_driver::ComputePipeline,
    }

    impl DoublingKernel {
        fn new(context: &ComputeContext, input: Vec<u32>) -> Self {
            const SHADER: &str = r#"
                @group(0) @binding(0)
                var<storage, read_write> data: array<u32>;

                @compute @workgroup_size(64)
                fn double_entry(@builtin(global_invocation_id) id: vec3<u32>) {
                    data[id.x] = data[id.x] * 2u;
                }
            "#;
            let gpu = context
                .gpu()
                .expect("DoublingKernel::new requires a GPU-backed ComputeContext");
            let shader = gpu.create_shader("double_entry", SHADER);
            let pipeline = gpu.create_compute_pipeline(&shader, "double_entry");
            let output = std::sync::Mutex::new(vec![0u32; input.len()]);
            Self {
                input,
                output,
                pipeline,
            }
        }
    }

    impl HybridKernel for DoublingKernel {
        fn run_cpu(&self, range: Range<usize>) {
            let mut output = self.output.lock().unwrap();
            for i in range {
                output[i] = self.input[i] * 2;
            }
        }

        async fn run_gpu(&self, context: &ComputeContext, range: Range<usize>) {
            let gpu = context.gpu().expect("run_hybrid only calls run_gpu when a GPU backend exists");
            let slice = &self.input[range.clone()];
            let bytes: Vec<u8> = slice.iter().flat_map(|v| v.to_le_bytes()).collect();
            let buffer = gpu.allocate_buffer(bytes.len(), meridian_gpu_driver::BufferUsage::Storage);
            gpu.write_buffer(&buffer, &bytes);
            let workgroups = (range.len() as u32).div_ceil(64).max(1);
            gpu.dispatch(&self.pipeline, &buffer, workgroups);
            let result_bytes = gpu.read_buffer(&buffer).await;
            let results = result_bytes
                .chunks_exact(4)
                .map(|c| u32::from_le_bytes(c.try_into().unwrap()));
            let mut output = self.output.lock().unwrap();
            for (offset, value) in results.enumerate() {
                output[range.start + offset] = value;
            }
        }
    }

    /// Needs a real adapter; some CI/sandboxed environments have none —
    /// skip rather than fail, matching every other GPU-touching test in
    /// this workspace.
    #[tokio::test]
    async fn run_hybrid_splits_work_across_cpu_and_gpu_and_merges_results() {
        let context = match ComputeContext::new().with_gpu().await {
            Ok(ctx) => ctx,
            Err(err) => {
                eprintln!("skipping: no GPU device available ({err})");
                return;
            }
        };
        let n = 200;
        let input: Vec<u32> = (0..n as u32).collect();
        let kernel = Arc::new(DoublingKernel::new(&context, input.clone()));

        context
            .run_hybrid(kernel.clone(), n, BackendSplit::Ratio(0.5))
            .await;

        let output = kernel.output.lock().unwrap();
        for i in 0..n {
            assert_eq!(
                output[i],
                input[i] * 2,
                "index {i} should be doubled regardless of which backend processed it"
            );
        }
    }
}

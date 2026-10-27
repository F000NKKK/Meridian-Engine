//! Tests of this module.

use std::ops::Range;
use std::sync::Arc;

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
#[cfg(feature = "hw-tests")]
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
        let gpu = context
            .gpu()
            .expect("run_hybrid only calls run_gpu when a GPU backend exists");
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
#[cfg(feature = "hw-tests")]
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

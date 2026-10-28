//! Tests of this module.

use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use super::*;
use crate::testing::{FakeGpu, SequentialDevice};

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

fn cpu_context() -> ComputeContext {
    ComputeContext::new(Arc::new(SequentialDevice))
}

#[test]
fn support_reports_no_gpu_by_default_and_the_fake_gpus_name_once_added() {
    let context = cpu_context();
    assert!(!context.has_gpu());
    assert_eq!(context.support().gpu_device_name, None);
    assert_eq!(context.support().threads, 1);

    let context = context.with_gpu(Arc::new(FakeGpu::named("fake adapter")));
    assert!(context.has_gpu());
    assert_eq!(
        context.support().gpu_device_name.as_deref(),
        Some("fake adapter")
    );
}

#[test]
fn gpu_downcasts_to_its_concrete_type_only() {
    #[derive(Debug)]
    struct OtherGpu;
    impl GpuBackend for OtherGpu {
        fn device_name(&self) -> String {
            "other".to_owned()
        }
        fn as_any(&self) -> &dyn std::any::Any {
            self
        }
    }
    let context = cpu_context().with_gpu(Arc::new(FakeGpu::named("fake")));
    assert!(context.gpu::<FakeGpu>().is_some());
    assert!(context.gpu::<OtherGpu>().is_none());
    assert!(cpu_context().gpu::<FakeGpu>().is_none());
}

#[test]
fn allocate_buffer_is_zeroed() {
    let buffer = cpu_context().allocate_buffer(8);
    assert_eq!(buffer.as_slice(), &[0; 8]);
}

#[test]
fn parallel_for_below_threshold_still_visits_every_index() {
    let ctx = cpu_context().with_parallel_threshold(1_000_000); // force sequential path
    let counter = AtomicUsize::new(0);
    ctx.parallel_for(10, |_| {
        counter.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(counter.load(Ordering::SeqCst), 10);
}

#[test]
fn parallel_for_above_threshold_still_visits_every_index() {
    let ctx = cpu_context().with_parallel_threshold(1); // force parallel path
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
    let scheduler = ComputeScheduler::with_parallel_threshold(Arc::new(SequentialDevice), 1);
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
    let context = cpu_context(); // no with_gpu()
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

/// Records which indices each half got; the GPU half writes `1`, the CPU half `0`.
struct SplitRecorder {
    owner: std::sync::Mutex<Vec<Option<u8>>>,
}

impl HybridKernel for SplitRecorder {
    fn run_cpu(&self, range: Range<usize>) {
        let mut owner = self.owner.lock().unwrap();
        for index in range {
            owner[index] = Some(0);
        }
    }

    #[expect(
        clippy::unused_async_trait_impl,
        reason = "the fake records on the CPU; the trait method is async"
    )]
    async fn run_gpu(&self, context: &ComputeContext, range: Range<usize>) {
        assert!(
            context.gpu::<FakeGpu>().is_some(),
            "run_gpu needs the GPU backend"
        );
        let mut owner = self.owner.lock().unwrap();
        for index in range {
            owner[index] = Some(1);
        }
    }
}

#[tokio::test]
async fn run_hybrid_hands_each_index_to_exactly_one_backend() {
    let context = cpu_context().with_gpu(Arc::new(FakeGpu::named("fake")));
    for split in [
        BackendSplit::CpuOnly,
        BackendSplit::GpuOnly,
        BackendSplit::Ratio(0.25),
    ] {
        let kernel = Arc::new(SplitRecorder {
            owner: std::sync::Mutex::new(vec![None; 100]),
        });
        context.run_hybrid(kernel.clone(), 100, split).await;
        let owner = kernel.owner.lock().unwrap();
        assert!(
            owner.iter().all(Option::is_some),
            "{split:?} left an index unprocessed"
        );
        let on_gpu = owner.iter().filter(|o| **o == Some(1)).count();
        assert_eq!(on_gpu, split.gpu_item_count(100, true), "{split:?}");
    }
}

#[tokio::test]
async fn run_hybrid_of_nothing_calls_nothing() {
    let kernel = Arc::new(PanicsOnGpuKernel {
        cpu_visited: std::sync::Mutex::new(Vec::new()),
    });
    cpu_context()
        .run_hybrid(kernel.clone(), 0, BackendSplit::GpuOnly)
        .await;
    assert!(kernel.cpu_visited.lock().unwrap().is_empty());
}

#[test]
fn parallel_ranges_cover_every_index_exactly_once() {
    for threshold in [1, usize::MAX] {
        let ctx = cpu_context().with_parallel_threshold(threshold);
        let seen = std::sync::Mutex::new(vec![0u8; 5_000]);
        ctx.parallel_ranges(5_000, |range| {
            let mut seen = seen.lock().unwrap();
            for i in range {
                seen[i] += 1;
            }
        });
        assert!(
            seen.lock().unwrap().iter().all(|&n| n == 1),
            "threshold {threshold}"
        );
    }
}

#[test]
fn parallel_ranges_with_no_items_never_calls_work() {
    cpu_context().parallel_ranges(0, |_| panic!("no work expected"));
}

#[test]
fn default_threshold_keeps_a_thousand_items_in_one_sequential_range() {
    let calls = AtomicUsize::new(0);
    cpu_context().parallel_ranges(1_000, |range| {
        assert_eq!(range, 0..1_000);
        calls.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(calls.load(Ordering::SeqCst), 1);
}

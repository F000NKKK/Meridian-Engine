//! Tests of this crate.

use std::ops::Range;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use meridian_compute_app::testing::run_device_contract;
use meridian_compute_app::{ComputeContext, ComputeDevice};

use super::*;

#[test]
fn capabilities_report_at_least_one_cpu_thread_and_no_gpu() {
    let device = CpuComputeDevice::new();
    let caps = device.capabilities();
    assert!(caps.cpu.threads >= 1);
    assert!(caps.gpu.is_none());
    assert_eq!(device.support().gpu_device_name, None);
}

#[test]
fn cpu_device_passes_the_device_contract() {
    run_device_contract(CpuComputeDevice::new);
}

#[test]
fn dispatch_ranges_matches_sequential_sum() {
    let n = 50_000usize;
    let total = AtomicUsize::new(0);
    CpuComputeDevice::new().dispatch_ranges(n, &|range| {
        total.fetch_add(range.sum::<usize>(), Ordering::Relaxed);
    });
    assert_eq!(total.load(Ordering::Relaxed), (0..n).sum::<usize>());
}

#[test]
fn context_over_the_cpu_device_runs_parallel_for_in_parallel_path() {
    let context = ComputeContext::new(Arc::new(CpuComputeDevice::new()));
    let total = AtomicUsize::new(0);
    context.parallel_for(10_000, |i| {
        total.fetch_add(i, Ordering::Relaxed);
    });
    assert_eq!(total.load(Ordering::Relaxed), (0..10_000).sum::<usize>());
}

/// Every `GpuComputeDevice` test needs a real adapter; some CI/
/// sandboxed environments have none. Skip rather than fail — this
/// validates this crate's own GPU-dispatch API, not that a GPU is
/// present.
#[cfg(feature = "hw-tests")]
async fn gpu_device_or_skip() -> Option<GpuComputeDevice> {
    match GpuComputeDevice::new().await {
        Ok(device) => Some(device),
        Err(err) => {
            eprintln!("skipping: no GPU device available ({err})");
            None
        }
    }
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn gpu_capabilities_report_a_real_device_name() {
    let Some(device) = gpu_device_or_skip().await else {
        return;
    };
    let caps = device.capabilities();
    assert!(caps.cpu.threads >= 1);
    let gpu = caps
        .gpu
        .expect("a constructed GpuComputeDevice always has a real adapter");
    assert_ne!(gpu.device_name, "");
}

/// The actual end-to-end proof: write data to a GPU buffer, run a
/// real compute shader over it, read the result back, and check it
/// matches what the shader says it does.
#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn gpu_dispatch_doubles_every_element() {
    let Some(device) = gpu_device_or_skip().await else {
        return;
    };

    const SHADER: &str = r#"
        @group(0) @binding(0)
        var<storage, read_write> data: array<u32>;

        @compute @workgroup_size(4)
        fn double_elements(@builtin(global_invocation_id) id: vec3<u32>) {
            data[id.x] = data[id.x] * 2u;
        }
    "#;

    let input: [u32; 4] = [1, 2, 3, 4];
    let bytes: Vec<u8> = input.iter().flat_map(|v| v.to_le_bytes()).collect();

    let buffer = device.allocate_buffer(bytes.len(), meridian_gpu_wgpu::BufferUsage::Storage);
    device.write_buffer(&buffer, &bytes);

    let shader = device.create_shader("double_elements", SHADER);
    let pipeline = device.create_compute_pipeline(&shader, "double_elements");
    device.dispatch(&pipeline, &buffer, 1);

    let result_bytes = device.read_buffer(&buffer).await;
    let result: Vec<u32> = result_bytes
        .as_chunks::<4>()
        .0
        .iter()
        .map(|c| u32::from_le_bytes(*c))
        .collect();
    assert_eq!(result, vec![2, 4, 6, 8]);
}

#[cfg(feature = "hw-tests")]
#[test]
fn gpu_device_passes_the_gpu_contract() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .expect("a current-thread runtime");
    let make = || runtime.block_on(GpuComputeDevice::new());
    if make().is_err() {
        eprintln!("skipping: no GPU device available");
        return;
    }
    meridian_compute_app::testing::run_gpu_contract(|| {
        make().expect("the adapter was there a moment ago")
    });
}

/// A real doubling kernel: `run_cpu` doubles in Rust, `run_gpu` dispatches a
/// WGSL compute shader that does the same; both write into one output by
/// index, so the hybrid dispatch is proven to run each half and merge them.
#[cfg(feature = "hw-tests")]
struct DoublingKernel {
    input: Vec<u32>,
    output: std::sync::Mutex<Vec<u32>>,
    pipeline: meridian_gpu_wgpu::ComputePipeline,
}

#[cfg(feature = "hw-tests")]
impl DoublingKernel {
    fn new(gpu: &GpuComputeDevice, input: Vec<u32>) -> Self {
        const SHADER: &str = r"
            @group(0) @binding(0)
            var<storage, read_write> data: array<u32>;

            @compute @workgroup_size(64)
            fn double_entry(@builtin(global_invocation_id) id: vec3<u32>) {
                data[id.x] = data[id.x] * 2u;
            }
        ";
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

#[cfg(feature = "hw-tests")]
impl meridian_compute_app::HybridKernel for DoublingKernel {
    fn run_cpu(&self, range: Range<usize>) {
        let mut output = self.output.lock().unwrap();
        for i in range {
            output[i] = self.input[i] * 2;
        }
    }

    async fn run_gpu(&self, context: &ComputeContext, range: Range<usize>) {
        let gpu = context
            .gpu::<GpuComputeDevice>()
            .expect("run_hybrid only calls run_gpu when a GPU backend exists");
        let bytes: Vec<u8> = self.input[range.clone()]
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        let buffer = gpu.allocate_buffer(bytes.len(), meridian_gpu_wgpu::BufferUsage::Storage);
        gpu.write_buffer(&buffer, &bytes);
        let workgroups = (range.len() as u32).div_ceil(64).max(1);
        gpu.dispatch(&self.pipeline, &buffer, workgroups);
        let result_bytes = gpu.read_buffer(&buffer).await;
        let results = result_bytes
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| u32::from_le_bytes(*c));
        let mut output = self.output.lock().unwrap();
        for (offset, value) in results.enumerate() {
            output[range.start + offset] = value;
        }
    }
}

#[cfg(feature = "hw-tests")]
#[tokio::test]
async fn run_hybrid_splits_work_across_cpu_and_gpu_and_merges_results() {
    use meridian_compute_app::BackendSplit;

    let Some(gpu) = gpu_device_or_skip().await else {
        return;
    };
    let kernel = Arc::new(DoublingKernel::new(&gpu, (0..200u32).collect()));
    let context = ComputeContext::new(Arc::new(CpuComputeDevice::new())).with_gpu(Arc::new(gpu));
    context
        .run_hybrid(kernel.clone(), 200, BackendSplit::Ratio(0.5))
        .await;
    let output = kernel.output.lock().unwrap();
    for (i, value) in output.iter().enumerate() {
        assert_eq!(*value, kernel.input[i] * 2, "index {i}");
    }
}

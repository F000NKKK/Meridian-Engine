//! Low-level compute dispatch abstraction (CPU SIMD backends, GPU compute queues/buffers). Knows nothing about physics, animation or rendering.
//!
//! Two independent backends. [`ComputeDevice`] (CPU) is real, not a stub:
//! [`ComputeDevice::dispatch_parallel`] runs work across real OS threads
//! via `std::thread::scope`, safe, no `unsafe`. [`GpuComputeDevice`] is
//! real too now, built on [`meridian_gpu_driver`] — the crate shared with
//! `graphics-driver` that owns the actual `wgpu` device/buffer/shader
//! mechanics (see that crate's module doc and
//! [ADR 011](../../../docs/adr/011-shared-gpu-driver-crate.md)); this
//! crate adds nothing on top but the compute-dispatch-shaped API
//! (`allocate_buffer`/`dispatch`) `compute-runtime::ComputeContext` uses
//! to reach it (see rule 5 in docs/dependency-rules.md: `compute-runtime`
//! is the sanctioned path to CPU-SIMD/GPU-compute for domain crates, not
//! this crate directly). A caller picks a backend explicitly
//! (`ComputeContext::new` for CPU-only, `ComputeContext::with_gpu` to add
//! the GPU path alongside it) — there's no automatic fallback between
//! them.
//!
//! **Async on genuine I/O, not on everything** — same policy as every
//! other driver crate (see
//! [ADR 009](../../../docs/adr/009-async-io-via-tokio.md)):
//! [`GpuComputeDevice::new`] (an OS/driver handshake) and
//! [`GpuComputeDevice::read_buffer`] (waiting on in-flight GPU work) are
//! real `async fn`s; `ComputeDevice`'s CPU path and GPU allocation/
//! dispatch-recording calls stay synchronous.

use meridian_platform_core::{BackendCapabilities, CpuCapabilities, GpuCapabilities};

/// Backend capability flags. Embeds `platform-core`'s [`CpuCapabilities`]
/// (the shape shared with `physics-driver::PhysicsBackend` and
/// `graphics-driver`/`audio-driver` equivalents) rather than redeclaring
/// `threads`; `gpu` is `None` for [`ComputeDevice`] (the CPU backend has
/// no GPU to report) and `Some(GpuCapabilities { .. })` for
/// [`GpuComputeDevice`] (real detected fields, via
/// `meridian_gpu_driver::Device`'s own `BackendCapabilities` impl).
#[derive(Debug, Clone, Default)]
pub struct ComputeCapabilities {
    pub cpu: CpuCapabilities,
    pub gpu: Option<GpuCapabilities>,
}

impl BackendCapabilities for ComputeCapabilities {
    fn cpu(&self) -> CpuCapabilities {
        self.cpu
    }

    fn gpu(&self) -> Option<GpuCapabilities> {
        self.gpu.clone()
    }
}

/// A CPU-backend compute buffer: owned bytes. [`GpuComputeDevice`] uses
/// `meridian_gpu_driver::Buffer` instead (a real device-memory handle) —
/// genuinely different representations for genuinely different backends,
/// not something worth unifying into one buffer type that would have to
/// paper over "sometimes owned bytes, sometimes a GPU handle."
#[derive(Debug, Clone, Default)]
pub struct ComputeBuffer {
    data: Vec<u8>,
}

impl ComputeBuffer {
    pub fn zeroed(byte_len: usize) -> Self {
        Self {
            data: vec![0u8; byte_len],
        }
    }

    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

/// The active compute backend. Only ever the CPU backend right now — see
/// the module doc.
#[derive(Debug, Clone)]
pub struct ComputeDevice {
    capabilities: ComputeCapabilities,
}

impl Default for ComputeDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputeDevice {
    pub fn new() -> Self {
        Self {
            capabilities: ComputeCapabilities {
                cpu: CpuCapabilities::detect(),
                gpu: None,
            },
        }
    }

    pub fn capabilities(&self) -> ComputeCapabilities {
        self.capabilities.clone()
    }

    pub fn allocate_buffer(&self, byte_len: usize) -> ComputeBuffer {
        ComputeBuffer::zeroed(byte_len)
    }

    /// Runs `work(i)` exactly once for every `i` in `0..count`, splitting
    /// the range into `capabilities().cpu.threads` contiguous chunks
    /// executed on real worker threads via `std::thread::scope` (borrowed
    /// `work`, no `'static`/`Box` needed). Always parallel — callers that
    /// want a small-batch/sequential cutoff apply that policy themselves
    /// (see `meridian-compute-runtime`'s `ComputeContext`, which is the
    /// intended caller); this is mechanism, not policy.
    pub fn dispatch_parallel(&self, count: usize, work: impl Fn(usize) + Sync) {
        if count == 0 {
            return;
        }
        let threads = self.capabilities.cpu.threads.max(1).min(count);
        let chunk = count.div_ceil(threads);
        let work = &work;
        std::thread::scope(|scope| {
            for t in 0..threads {
                let start = t * chunk;
                let end = (start + chunk).min(count);
                if start >= end {
                    continue;
                }
                scope.spawn(move || {
                    for i in start..end {
                        work(i);
                    }
                });
            }
        });
    }
}

/// The GPU compute backend: a real headless `wgpu` device (via
/// `meridian_gpu_driver::Device`), independent of [`ComputeDevice`]'s CPU
/// path — a caller constructs whichever backend(s) it needs (see
/// `compute-runtime::ComputeContext::with_gpu`, the intended caller).
/// `Clone` is cheap — see `meridian_gpu_driver::Device`'s own doc comment.
#[derive(Debug, Clone)]
pub struct GpuComputeDevice {
    device: meridian_gpu_driver::Device,
}

/// Why [`GpuComputeDevice::new`] failed — a re-export of
/// `meridian_gpu_driver::DeviceError` under this crate's own name, so
/// callers one layer up (`compute-runtime::ComputeContext::with_gpu`)
/// only ever need to know this crate's error surface, not reach past it
/// into `gpu-driver` directly for a type name.
pub type GpuComputeError = meridian_gpu_driver::DeviceError;

impl GpuComputeDevice {
    /// Requests a headless GPU device — see
    /// `meridian_gpu_driver::Device::new`. A real `async fn`; see the
    /// module doc.
    pub async fn new() -> Result<Self, GpuComputeError> {
        Ok(Self {
            device: meridian_gpu_driver::Device::new().await?,
        })
    }

    pub fn capabilities(&self) -> ComputeCapabilities {
        ComputeCapabilities {
            cpu: CpuCapabilities::detect(),
            gpu: self.device.gpu(),
        }
    }

    pub fn allocate_buffer(
        &self,
        byte_len: usize,
        usage: meridian_gpu_driver::BufferUsage,
    ) -> meridian_gpu_driver::Buffer {
        self.device.create_buffer(byte_len, usage)
    }

    pub fn write_buffer(&self, buffer: &meridian_gpu_driver::Buffer, data: &[u8]) {
        self.device.write_buffer(buffer, data);
    }

    /// Reads `buffer` back to the CPU — waits on in-flight GPU work, a
    /// real `async fn`; see the module doc.
    pub async fn read_buffer(&self, buffer: &meridian_gpu_driver::Buffer) -> Vec<u8> {
        self.device.read_buffer(buffer).await
    }

    pub fn create_shader(&self, label: &str, wgsl_source: &str) -> meridian_gpu_driver::Shader {
        self.device.create_shader(label, wgsl_source)
    }

    pub fn create_compute_pipeline(
        &self,
        shader: &meridian_gpu_driver::Shader,
        entry_point: &str,
    ) -> meridian_gpu_driver::ComputePipeline {
        self.device.create_compute_pipeline(shader, entry_point)
    }

    /// Records and submits a single compute dispatch over `buffer` — a
    /// one-shot convenience over `meridian_gpu_driver`'s own
    /// `CommandBuffer` for the common "one pipeline, one buffer, submit"
    /// case; a caller that needs to batch multiple dispatches into one
    /// submission can still reach `meridian_gpu_driver::Device` directly
    /// for that (not exposed through this crate, since batching multiple
    /// dispatches together is dispatch *scheduling* policy —
    /// `compute-runtime`'s job per rule 11 in docs/dependency-rules.md,
    /// not this crate's).
    pub fn dispatch(
        &self,
        pipeline: &meridian_gpu_driver::ComputePipeline,
        buffer: &meridian_gpu_driver::Buffer,
        workgroups: u32,
    ) {
        let mut commands = self.device.create_command_buffer();
        commands.dispatch_compute(pipeline, buffer, workgroups);
        commands.submit();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn capabilities_report_at_least_one_cpu_thread_and_no_gpu() {
        let device = ComputeDevice::new();
        let caps = device.capabilities();
        assert!(caps.cpu.threads >= 1);
        assert!(caps.gpu.is_none());
    }

    #[test]
    fn buffer_zeroed_has_requested_length_and_zero_bytes() {
        let buf = ComputeBuffer::zeroed(16);
        assert_eq!(buf.len(), 16);
        assert!(buf.as_slice().iter().all(|&b| b == 0));
    }

    #[test]
    fn buffer_mut_slice_writes_are_visible() {
        let mut buf = ComputeBuffer::zeroed(4);
        buf.as_mut_slice()[2] = 42;
        assert_eq!(buf.as_slice(), &[0, 0, 42, 0]);
    }

    #[test]
    fn dispatch_parallel_zero_count_calls_work_zero_times() {
        let calls = AtomicUsize::new(0);
        ComputeDevice::new().dispatch_parallel(0, |_| {
            calls.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn dispatch_parallel_calls_every_index_exactly_once() {
        let n = 10_000;
        let seen: Mutex<Vec<bool>> = Mutex::new(vec![false; n]);
        ComputeDevice::new().dispatch_parallel(n, |i| {
            let mut seen = seen.lock().unwrap();
            assert!(!seen[i], "index {i} visited twice");
            seen[i] = true;
        });
        assert!(
            seen.lock().unwrap().iter().all(|&v| v),
            "every index must have been visited"
        );
    }

    #[test]
    fn dispatch_parallel_matches_sequential_sum() {
        let n = 50_000usize;
        let total = AtomicUsize::new(0);
        ComputeDevice::new().dispatch_parallel(n, |i| {
            total.fetch_add(i, Ordering::Relaxed);
        });
        let expected: usize = (0..n).sum();
        assert_eq!(total.load(Ordering::Relaxed), expected);
    }

    /// Every `GpuComputeDevice` test needs a real adapter; some CI/
    /// sandboxed environments have none. Skip rather than fail — this
    /// validates this crate's own GPU-dispatch API, not that a GPU is
    /// present.
    async fn gpu_device_or_skip() -> Option<GpuComputeDevice> {
        match GpuComputeDevice::new().await {
            Ok(device) => Some(device),
            Err(err) => {
                eprintln!("skipping: no GPU device available ({err})");
                None
            }
        }
    }

    #[tokio::test]
    async fn gpu_capabilities_report_a_real_device_name() {
        let Some(device) = gpu_device_or_skip().await else {
            return;
        };
        let caps = device.capabilities();
        assert!(caps.cpu.threads >= 1);
        let gpu = caps.gpu.expect("a constructed GpuComputeDevice always has a real adapter");
        assert!(!gpu.device_name.is_empty());
    }

    /// The actual end-to-end proof: write data to a GPU buffer, run a
    /// real compute shader over it, read the result back, and check it
    /// matches what the shader says it does.
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

        let buffer = device.allocate_buffer(bytes.len(), meridian_gpu_driver::BufferUsage::Storage);
        device.write_buffer(&buffer, &bytes);

        let shader = device.create_shader("double_elements", SHADER);
        let pipeline = device.create_compute_pipeline(&shader, "double_elements");
        device.dispatch(&pipeline, &buffer, 1);

        let result_bytes = device.read_buffer(&buffer).await;
        let result: Vec<u32> = result_bytes
            .chunks_exact(4)
            .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
            .collect();
        assert_eq!(result, vec![2, 4, 6, 8]);
    }
}

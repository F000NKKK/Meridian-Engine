//! Low-level compute dispatch abstraction (CPU SIMD backends, GPU compute queues/buffers). Knows nothing about physics, animation or rendering.
//!
//! Layer: infrastructure (adapter); bounded context: compute
//! (`craftbook.toml`, docs/context-map.md).
//!
//! The adapter for `compute-app`'s ports (ADR 019). Two independent
//! backends. [`CpuComputeDevice`] implements `ComputeDevice` and runs work
//! across real OS threads via `std::thread::scope`, safe, no `unsafe`.
//! [`GpuComputeDevice`] implements `GpuBackend` and is
//! real too now, built on [`meridian_gpu_wgpu`] — the crate shared with
//! `rendering-wgpu` that owns the actual `wgpu` device/buffer/shader
//! mechanics (see that crate's module doc and
//! [ADR 011](../../../docs/adr/011-shared-gpu-driver-crate.md)); this
//! crate adds nothing on top but the compute-dispatch-shaped API
//! (`allocate_buffer`/`dispatch`) kernel adapters use. The composition
//! root builds the devices and hands them to a `ComputeContext`
//! (`ComputeContext::new` for CPU-only, `with_gpu` to add the GPU path
//! alongside it) — there is no automatic fallback between them.
//!
//! Design notes: docs/notes/meridian-compute-wgpu.md (`lib`).

#![forbid(unsafe_code)]

use std::any::Any;
use std::ops::Range;
use std::sync::Arc;

use meridian_compute_app::{ComputeContext, ComputeDevice, ComputeSupport, GpuBackend};
use meridian_platform_winit::{BackendCapabilities, CpuCapabilities, GpuCapabilities};

/// Backend capability flags (the platform's report shape; the port's is
/// [`ComputeSupport`]). Embeds `platform-winit`'s [`CpuCapabilities`]
/// (the shape shared with future
/// `rendering-wgpu`/`audio-cpal` equivalents) rather than redeclaring
/// `threads`; `gpu` is `None` for [`ComputeDevice`] (the CPU backend has
/// no GPU to report) and `Some(GpuCapabilities { .. })` for
/// [`GpuComputeDevice`] (real detected fields, via
/// `meridian_gpu_wgpu::Device`'s own `BackendCapabilities` impl).
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

/// The CPU compute backend: implements the [`ComputeDevice`] port with real OS
/// threads.
#[derive(Debug, Clone)]
pub struct CpuComputeDevice {
    capabilities: ComputeCapabilities,
}

impl Default for CpuComputeDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuComputeDevice {
    /// A device sized to this machine's CPU ([`CpuCapabilities::detect`]).
    #[must_use]
    pub fn new() -> Self {
        Self {
            capabilities: ComputeCapabilities {
                cpu: CpuCapabilities::detect(),
                gpu: None,
            },
        }
    }

    /// What this backend reports through the platform's capability shape.
    #[must_use]
    pub fn capabilities(&self) -> ComputeCapabilities {
        self.capabilities.clone()
    }
}

impl ComputeDevice for CpuComputeDevice {
    fn support(&self) -> ComputeSupport {
        ComputeSupport {
            threads: self.capabilities.cpu.threads.max(1),
            gpu_device_name: None,
        }
    }

    /// Splits the range into `threads` contiguous chunks executed on real
    /// worker threads via `std::thread::scope` (borrowed `work`, no `'static`).
    /// Always parallel: the cutoff for small batches is the context's policy.
    fn dispatch_ranges(&self, count: usize, work: &(dyn Fn(Range<usize>) + Sync)) {
        if count == 0 {
            return;
        }
        let threads = self.capabilities.cpu.threads.max(1).min(count);
        let chunk = count.div_ceil(threads);
        let ranges = (0..threads)
            .map(|t| (t * chunk)..((t + 1) * chunk).min(count))
            .filter(|range| range.start < range.end);
        std::thread::scope(|scope| {
            for range in ranges {
                scope.spawn(move || work(range));
            }
        });
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

/// The GPU compute backend: a real headless `wgpu` device (via
/// `meridian_gpu_wgpu::Device`), independent of [`ComputeDevice`]'s CPU
/// path — a caller constructs whichever backend(s) it needs (see
/// `compute-app::ComputeContext::with_gpu`, the intended caller).
/// `Clone` is cheap — see `meridian_gpu_wgpu::Device`'s own doc comment.
#[derive(Debug, Clone)]
pub struct GpuComputeDevice {
    device: meridian_gpu_wgpu::Device,
}

/// Why [`GpuComputeDevice::new`] failed — a re-export of
/// `meridian_gpu_wgpu::DeviceError` under this crate's own name, so
/// callers one layer up (`compute-app::ComputeContext::with_gpu`)
/// only ever need to know this crate's error surface, not reach past it
/// into `gpu-wgpu` directly for a type name.
pub type GpuComputeError = meridian_gpu_wgpu::DeviceError;

impl GpuComputeDevice {
    /// Requests a headless GPU device — see
    /// `meridian_gpu_wgpu::Device::new`. A real `async fn`; see the
    /// module doc.
    pub async fn new() -> Result<Self, GpuComputeError> {
        Ok(Self {
            device: meridian_gpu_wgpu::Device::new().await?,
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
        usage: meridian_gpu_wgpu::BufferUsage,
    ) -> meridian_gpu_wgpu::Buffer {
        self.device.create_buffer(byte_len, usage)
    }

    pub fn write_buffer(&self, buffer: &meridian_gpu_wgpu::Buffer, data: &[u8]) {
        self.device.write_buffer(buffer, data);
    }

    /// Reads `buffer` back to the CPU — waits on in-flight GPU work, a
    /// real `async fn`; see the module doc.
    pub async fn read_buffer(&self, buffer: &meridian_gpu_wgpu::Buffer) -> Vec<u8> {
        self.device.read_buffer(buffer).await
    }

    pub fn create_shader(&self, label: &str, wgsl_source: &str) -> meridian_gpu_wgpu::Shader {
        self.device.create_shader(label, wgsl_source)
    }

    pub fn create_compute_pipeline(
        &self,
        shader: &meridian_gpu_wgpu::Shader,
        entry_point: &str,
    ) -> meridian_gpu_wgpu::ComputePipeline {
        self.device.create_compute_pipeline(shader, entry_point)
    }

    /// Records and submits a single compute dispatch over `buffer` — a
    /// one-shot convenience over `meridian_gpu_wgpu`'s own
    /// `CommandBuffer` for the common "one pipeline, one buffer, submit"
    /// case; a caller that needs to batch multiple dispatches into one
    /// submission can still reach `meridian_gpu_wgpu::Device` directly
    /// for that (not exposed through this crate, since batching multiple
    /// dispatches together is dispatch *scheduling* policy —
    /// `compute-app`'s job per rule 11 in docs/dependency-rules.md,
    /// not this crate's).
    pub fn dispatch(
        &self,
        pipeline: &meridian_gpu_wgpu::ComputePipeline,
        buffer: &meridian_gpu_wgpu::Buffer,
        workgroups: u32,
    ) {
        let mut commands = self.device.create_command_buffer();
        commands.dispatch_compute(pipeline, buffer, workgroups);
        commands.submit();
    }

    /// The raw `meridian_gpu_wgpu::Device` — an escape hatch for a
    /// kernel author (e.g. `gac-wgpu`'s `Fixed` arithmetic kernels)
    /// that needs a bind group/dispatch shape beyond
    /// [`GpuComputeDevice::dispatch`]'s single-buffer convenience (a
    /// kernel reading from one buffer and writing to another, for
    /// instance) — see [`meridian_gpu_wgpu::Device::create_bind_group`]/
    /// [`meridian_gpu_wgpu::CommandBuffer::dispatch_compute_with_bind_group`].
    pub fn gpu_driver_device(&self) -> &meridian_gpu_wgpu::Device {
        &self.device
    }
}

/// A CPU-only [`ComputeContext`] over a [`CpuComputeDevice`]: the composition
/// root's one-liner for the default backend.
#[must_use]
pub fn cpu_context() -> ComputeContext {
    ComputeContext::new(Arc::new(CpuComputeDevice::new()))
}

/// [`cpu_context`] plus a GPU backend. A real `async fn` (acquiring the device
/// is genuine I/O).
///
/// # Errors
///
/// [`GpuComputeError`] when there is no adapter: there is no silent fallback
/// to CPU-only, so a caller that wants the GPU finds out here.
pub async fn gpu_context() -> Result<ComputeContext, GpuComputeError> {
    Ok(cpu_context().with_gpu(Arc::new(GpuComputeDevice::new().await?)))
}

impl GpuBackend for GpuComputeDevice {
    fn device_name(&self) -> String {
        self.capabilities()
            .gpu
            .map_or_else(|| "unknown GPU".to_owned(), |gpu| gpu.device_name)
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests;

//! The GPU compute backend.

use std::any::Any;

use meridian_compute_app::GpuBackend;
use meridian_platform_winit::{BackendCapabilities, CpuCapabilities};

use crate::ComputeCapabilities;

/// The GPU compute backend: a real headless `wgpu` device (via
/// `meridian_gpu_wgpu::Device`), independent of [`ComputeDevice`](meridian_compute_app::ComputeDevice)'s CPU
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

    /// Reports the CPU and GPU resources exposed by this backend.
    pub fn capabilities(&self) -> ComputeCapabilities {
        ComputeCapabilities {
            cpu: CpuCapabilities::detect(),
            gpu: self.device.gpu(),
        }
    }

    /// Allocates a GPU buffer of `byte_len` bytes with `usage` flags.
    pub fn allocate_buffer(
        &self,
        byte_len: usize,
        usage: meridian_gpu_wgpu::BufferUsage,
    ) -> meridian_gpu_wgpu::Buffer {
        self.device.create_buffer(byte_len, usage)
    }

    /// Uploads `data` to the start of `buffer`.
    pub fn write_buffer(&self, buffer: &meridian_gpu_wgpu::Buffer, data: &[u8]) {
        self.device.write_buffer(buffer, data);
    }

    /// Reads `buffer` back to the CPU — waits on in-flight GPU work, a
    /// real `async fn`; see the module doc.
    pub async fn read_buffer(&self, buffer: &meridian_gpu_wgpu::Buffer) -> Vec<u8> {
        self.device.read_buffer(buffer).await
    }

    /// Compiles WGSL source into a shader module.
    pub fn create_shader(&self, label: &str, wgsl_source: &str) -> meridian_gpu_wgpu::Shader {
        self.device.create_shader(label, wgsl_source)
    }

    /// Creates a compute pipeline for `shader` and its `entry_point`.
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

//! The capability flags a compute backend reports.

use meridian_platform_winit::{BackendCapabilities, CpuCapabilities, GpuCapabilities};

/// Backend capability flags (the platform's report shape; the port's is
/// [`ComputeSupport`](meridian_compute_app::ComputeSupport)). Embeds `platform-winit`'s [`CpuCapabilities`]
/// (the shape shared with future
/// `rendering-wgpu`/`audio-cpal` equivalents) rather than redeclaring
/// `threads`; `gpu` is `None` for [`ComputeDevice`](meridian_compute_app::ComputeDevice) (the CPU backend has
/// no GPU to report) and `Some(GpuCapabilities { .. })` for
/// [`GpuComputeDevice`](crate::GpuComputeDevice) (real detected fields, via
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

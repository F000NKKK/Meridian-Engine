//! Low-level compute dispatch abstraction (CPU SIMD backends, GPU compute queues/buffers). Knows nothing about physics, animation or rendering.

/// A dispatchable unit of CPU-SIMD or GPU-compute work.
#[derive(Debug, Clone, Default)]
pub struct ComputeKernel {
    pub name: &'static str,
}

/// A device-visible compute buffer.
#[derive(Debug, Clone, Copy, Default)]
pub struct ComputeBuffer {
    pub byte_len: usize,
}

/// Backend capability flags (CPU SIMD width, GPU compute support).
#[derive(Debug, Clone, Copy, Default)]
pub struct ComputeCapabilities {
    pub gpu_compute: bool,
}

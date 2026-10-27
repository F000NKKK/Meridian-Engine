//! Shared low-level `wgpu` device/buffer/shader/compute-pipeline mechanics underneath `rendering-wgpu` and `compute-wgpu`. Knows nothing about scenes, materials, render passes/swapchains, or compute-dispatch scheduling policy — see docs/adr/011-shared-gpu-driver-crate.md.
//!
//! Layer: infrastructure (adapter); bounded context: compute
//! (`craftbook.toml`, docs/context-map.md).
//!
//! Both `rendering-wgpu` (rendering) and `compute-wgpu` (general GPU
//! compute) need the same underlying primitives: acquire an adapter/
//! device, allocate/read/write buffers, compile a shader, build a compute
//! pipeline and dispatch it. Duplicating that wgpu-wrapping code in both
//! crates (which is what an earlier pass of this workspace did) is
//! exactly the kind of cross-crate duplication CLAUDE.md's "don't drag
//! another crate's logic into your own" rule exists to catch — so it
//! lives here once instead, the same shape `meridian-gac-wgpu` already
//! established for "two crates independently need the same underlying
//! thing" (see [ADR 007](../../../docs/adr/007-batch-transforms-via-compute.md)),
//! just at the driver layer instead of the domain layer.
//!
//! **Windowed surface creation is split deliberately.** [`Device::new_windowed`]
//! only does the adapter/device *handshake* with `compatible_surface`
//! set, and hands back the raw `wgpu::Surface` — it does not own
//! `SurfaceConfiguration`/swapchain lifecycle/present, which is a
//! graphics-specific concern `rendering-wgpu` builds on top (its own
//! `Surface` type). `compute-wgpu` never touches this constructor at
//! all — it only ever needs [`Device::new`] (headless).
//!
//! Design notes: docs/notes/meridian-gpu-wgpu.md (`lib`).

#![forbid(unsafe_code)]

mod bind_group;
mod buffer;
mod buffer_usage;
mod command_buffer;
mod compute_pipeline;
mod devices;
mod sampled_texture;
mod sampler;
mod shader;
#[cfg(test)]
mod tests;
mod texture;

pub use bind_group::BindGroup;
pub use buffer::Buffer;
pub use buffer_usage::BufferUsage;
pub use command_buffer::CommandBuffer;
pub use compute_pipeline::ComputePipeline;
pub use devices::*;
pub use sampled_texture::SampledTexture;
pub use sampler::Sampler;
pub use shader::Shader;
pub use texture::Texture;

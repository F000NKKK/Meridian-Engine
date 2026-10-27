//! The `wgpu` rendering adapter: device, surface, pipelines, and the recording of scenes, sprites and bloom into GPU passes.
//!
//! Layer: infrastructure (adapter); bounded context: rendering
//! (`craftbook.toml`, docs/context-map.md).
//!
//! Real, backed by `wgpu` via [`meridian_gpu_wgpu`] — the crate shared
//! with `compute-wgpu` that owns the actual wgpu device/buffer/shader
//! mechanics (see that crate's module doc and
//! [ADR 011](../../../docs/adr/011-shared-gpu-driver-crate.md)); this
//! crate adds only what's graphics-specific on top: [`Surface`]
//! (swapchain configuration/lifecycle), [`RenderPipeline`]
//! (vertex+fragment, with an optional [`DepthTexture`]), and
//! [`RenderPass`] recording. General GPU *compute* dispatch (a compute
//! pipeline bound to a buffer, unrelated to rendering) is
//! `compute-wgpu`'s job now, not this crate's — see that crate for it.
//!
//! Both headless ([`Device::new`], no `compatible_surface`) and windowed
//! ([`Device::new_windowed`], a real swapchain [`Surface`]) construction
//! exist. This crate stays `winit`-agnostic even for the windowed path —
//! [`Device::new_windowed`] takes `impl Into<wgpu::SurfaceTarget<'static>>`,
//! a `wgpu`-defined bound `Arc<winit::window::Window>` already satisfies
//! — see `platform-winit::Window::surface_target` and
//! [ADR 010](../../../docs/adr/010-windowing-via-winit.md).
//!
//! Design notes: docs/notes/meridian-rendering-wgpu.md (`lib`).

#![forbid(unsafe_code)]

mod acquire_frame_error;
mod bloom;
mod color_format;
mod command_buffer;
mod depth_texture;
mod devices;
mod gpu_frame_recorder;
mod gpu_texture_store;
mod render_pass;
mod render_pipeline;
mod shader_stages;
mod sprites;
mod submission;
mod surface;
mod surface_frame;
#[cfg(test)]
mod tests;
mod vertex;

pub use acquire_frame_error::AcquireFrameError;
pub use bloom::{BloomInput, BloomPass};
pub use color_format::ColorFormat;
pub use command_buffer::CommandBuffer;
pub use depth_texture::DepthTexture;
pub use devices::*;
pub use gpu_frame_recorder::GpuFrameRecorder;
pub use gpu_texture_store::GpuTextureStore;
// `Buffer`/`BindGroup` are re-exported so callers that already reach
// this crate's API (which returns/consumes them directly, e.g.
// `create_buffer`/`create_uniform_bind_group`) never need their own
// edge to `gpu-wgpu` just to name the type — the same
// resource-type-naming precedent as `gac-wgpu`'s direct `gpu-wgpu`
// dependency (ADR 011), applied by re-export instead of a second edge.
pub use meridian_gpu_wgpu::{
    BindGroup, Buffer, BufferUsage, DeviceError, SampledTexture, Sampler, Texture,
};
pub use render_pass::RenderPass;
pub use render_pipeline::RenderPipeline;
pub use shader_stages::ShaderStages;
pub use sprites::{PreparedSprites, SPRITE_SHADER_WGSL, SPRITE_VERTEX_BYTES, SpriteRenderer};
pub use submission::{
    DEFAULT_SHADOW_MAP_SIZE, DrawBuffers, EMISSIVE_EXTRACT_SHADER_WGSL, MAX_LIGHTS, PreparedScene,
    SHADOW_DEPTH_SHADER_WGSL, SceneAssets, SceneRenderer, TEXTURED_SHADER_WGSL, TextureRegistry,
    UNLIT_SHADER_WGSL, submit_scene3d,
};
pub use surface::Surface;
pub use surface_frame::SurfaceFrame;
pub use vertex::*;

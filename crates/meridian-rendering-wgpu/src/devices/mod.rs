//! The graphics device and its resource-creation methods.

mod bind_groups;
mod device;
mod environment_textures;
mod fullscreen_pipelines;
mod fullscreen_vertex;
mod half_float;
mod premultiplied_pipelines;
mod render_pipelines;
mod textures;

pub use device::Device;
pub(crate) use fullscreen_vertex::FULLSCREEN_VERTEX_WGSL;

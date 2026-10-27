//! The shader entry points of a render pipeline.

use meridian_gpu_wgpu::Shader;

/// A shader module and the names of its vertex and fragment entry points.
/// Both are entry points of the same `shader` module (a WGSL convention,
/// not a `wgpu` requirement).
#[derive(Debug, Clone, Copy)]
pub struct ShaderStages<'a> {
    /// The compiled module.
    pub shader: &'a Shader,
    /// The vertex stage's entry point.
    pub vertex_entry: &'a str,
    /// The fragment stage's entry point.
    pub fragment_entry: &'a str,
}

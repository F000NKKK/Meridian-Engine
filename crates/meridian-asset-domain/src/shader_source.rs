//! Shader source text.

/// Shader source, undecoded beyond a UTF-8 check — compiling it into a
/// `rendering-wgpu` shader module is that crate's job, not this one's.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ShaderSource {
    pub source: String,
}

//! Color attachment formats.

/// A color texture format callers can name without depending on `wgpu`
/// directly — the same "small enum, extend additively" shape as
/// [`VertexFormat`](crate::VertexFormat). Threading an explicit `ColorFormat` through
/// [`Device::create_offscreen_color_texture`](crate::Device::create_offscreen_color_texture) and its matching pipeline
/// constructors ([`Device::create_fullscreen_pipeline_for_offscreen`](crate::Device::create_fullscreen_pipeline_for_offscreen),
/// [`Device::create_render_pipeline_for_offscreen`](crate::Device::create_render_pipeline_for_offscreen)) is what makes an
/// offscreen texture's format and the pipeline that renders into it
/// *one* value instead of two independently-hardcoded assumptions that
/// can silently drift apart — the wgpu validation error this replaces
/// ("`RenderPass` uses textures with format X but the `RenderPipeline`
/// uses Y") is exactly what happens when they do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorFormat {
    /// 8-bit sRGB RGBA — what [`Device::create_texture_2d`](crate::Device::create_texture_2d) and
    /// sprite targets use.
    SrgbRgba8,
    /// 16-bit float RGBA, linear and unclamped: the scene and bloom targets
    /// (ADR 034), so light brighter than 1.0 survives until tone mapping.
    HdrRgba16F,
}

impl ColorFormat {
    pub(crate) fn to_wgpu(self) -> wgpu::TextureFormat {
        match self {
            ColorFormat::SrgbRgba8 => wgpu::TextureFormat::Rgba8UnormSrgb,
            ColorFormat::HdrRgba16F => wgpu::TextureFormat::Rgba16Float,
        }
    }
}

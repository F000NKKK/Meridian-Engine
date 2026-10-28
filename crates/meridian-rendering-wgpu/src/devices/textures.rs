//! Texture and sampler creation on the device.

use crate::{ColorFormat, DepthTexture, Device, Sampler, Texture};

impl Device {
    /// A `Depth32Float` depth/stencil-comparable texture (full float
    /// precision, no stencil since nothing here uses one yet), for
    /// [`CommandBuffer::begin_render_pass`](crate::CommandBuffer::begin_render_pass)'s `depth` parameter.
    pub fn create_depth_texture(&self, width: u32, height: u32) -> DepthTexture {
        let texture = self.0.create_texture(
            width,
            height,
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::RENDER_ATTACHMENT,
        );
        DepthTexture { texture }
    }

    /// A `Depth32Float` texture usable *both* as a render target (for a
    /// depth-only pass — see [`CommandBuffer::begin_shadow_pass`](crate::CommandBuffer::begin_shadow_pass)) and
    /// as a sampled `texture_depth_2d` (for the shadow lookup in a later
    /// lit-shader pass) — unlike [`create_depth_texture`](Self::create_depth_texture)'s
    /// plain [`DepthTexture`](crate::DepthTexture), which is only ever a render target
    /// (nothing samples the main pass's own depth buffer). `width` by
    /// `height` texels: the shadow atlas is a grid of square tiles (ADR 041).
    pub fn create_shadow_map_texture(&self, width: u32, height: u32) -> Texture {
        self.0.create_texture(
            width,
            height,
            wgpu::TextureFormat::Depth32Float,
            wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        )
    }

    /// A comparison sampler for shadow lookups — see
    /// [`meridian_gpu_wgpu::Device::create_comparison_sampler`]'s own
    /// doc for why a plain [`create_sampler`](Self::create_sampler)
    /// can't be reused here.
    pub fn create_comparison_sampler(&self) -> Sampler {
        self.0.create_comparison_sampler()
    }

    /// An `Rgba8UnormSrgb`, sampleable color texture — the shape a
    /// decoded `asset-domain::ImageAsset` (already RGBA8) uploads into for
    /// use as a material's albedo. sRGB, matching this crate's swapchain
    /// format choice (see [`Device::new_windowed`](crate::Device::new_windowed)'s doc comment), so a
    /// texture authored in sRGB (almost every image file is) reads back
    /// linear in the shader without a manual gamma-correction step.
    pub fn create_texture_2d(&self, width: u32, height: u32) -> Texture {
        self.0.create_texture(
            width,
            height,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        )
    }

    /// [`create_texture_2d`](Self::create_texture_2d) in `Rgba8Unorm`: no
    /// sRGB decode on sampling, for maps that hold numbers (a
    /// metallic-roughness map, ADR 031) rather than colours.
    #[must_use]
    pub fn create_linear_texture_2d(&self, width: u32, height: u32) -> Texture {
        self.0.create_texture(
            width,
            height,
            wgpu::TextureFormat::Rgba8Unorm,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
        )
    }

    /// Uploads tightly-packed RGBA8 pixel data to `texture`'s full
    /// extent — see [`meridian_gpu_wgpu::Device::write_texture`].
    pub fn write_texture(&self, texture: &Texture, rgba8_data: &[u8]) {
        self.0.write_texture(texture, rgba8_data);
    }

    /// A linear-filtered, repeat-addressed sampler (for tiled surface
    /// UVs) — see [`meridian_gpu_wgpu::Device::create_sampler`].
    pub fn create_sampler(&self) -> Sampler {
        self.0.create_sampler()
    }

    /// A linear-filtered, clamp-to-edge sampler (for full-screen post-
    /// process passes) — see
    /// [`meridian_gpu_wgpu::Device::create_clamp_sampler`].
    pub fn create_clamp_sampler(&self) -> Sampler {
        self.0.create_clamp_sampler()
    }

    /// A sampleable *and* renderable color texture
    /// (`TEXTURE_BINDING | RENDER_ATTACHMENT`, in `format`) — the shape a
    /// post-process pass's intermediate target needs: something to
    /// render into, then read back from in the next pass.
    /// [`create_texture_2d`](Self::create_texture_2d) is sampleable but
    /// not renderable (an uploaded asset texture is never a render
    /// target); this is that plus render-attachment usage.
    pub fn create_offscreen_color_texture(
        &self,
        width: u32,
        height: u32,
        format: ColorFormat,
    ) -> Texture {
        self.0.create_texture(
            width,
            height,
            format.to_wgpu(),
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::RENDER_ATTACHMENT,
        )
    }
}

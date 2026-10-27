//! Texture and sampler creation on the device.

use crate::{Device, Sampler, Texture};

impl Device {
    /// A 2D texture in `format`, with `usage` flags the caller picks
    /// (depth/stencil formats support a narrower usage set than color
    /// formats on some backends, so this doesn't guess). `graphics-driver`'s
    /// `DepthTexture` is a thin wrapper calling this with `Depth32Float` +
    /// `RENDER_ATTACHMENT`; a color texture at `Rgba8Unorm` +
    /// `TEXTURE_BINDING | COPY_DST | COPY_SRC` is the other common case.
    pub fn create_texture(
        &self,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Texture {
        let raw = self.device.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });
        let view = raw.create_view(&wgpu::TextureViewDescriptor::default());
        Texture {
            raw,
            view,
            width,
            height,
        }
    }

    /// Uploads `rgba8_data` (tightly packed, `width * height * 4` bytes)
    /// to `texture`'s full extent. The counterpart to [`write_buffer`](Self::write_buffer)
    /// for textures — `wgpu`'s `write_texture` needs the row/image byte
    /// layout spelled out explicitly (no implicit tight-packing the way
    /// buffer writes get), so this fills that in for the common
    /// tightly-packed-RGBA8 case.
    pub fn write_texture(&self, texture: &Texture, rgba8_data: &[u8]) {
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture.raw,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba8_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texture.width * 4),
                rows_per_image: Some(texture.height),
            },
            wgpu::Extent3d {
                width: texture.width,
                height: texture.height,
                depth_or_array_layers: 1,
            },
        );
    }

    /// A linear-filtered, clamp-to-edge sampler — the standard default
    /// for sampling an albedo texture (no mipmaps, no wrapping beyond the
    /// texture's own edges). Not parameterized yet: a second sampler
    /// configuration (wrapping, nearest-neighbor, ...) is additive future
    /// work once a concrete material needs one.
    /// `Repeat` addressing, not `ClampToEdge`: a UV that tiles a texture
    /// across a surface (a floor's checkerboard, say) needs values
    /// outside `[0,1]` to wrap back around, not smear the edge texel —
    /// `ClampToEdge` turns every tile beyond the first into a stretched
    /// streak of edge pixels, which reads as stripes, not a repeating
    /// pattern. `Repeat` is exactly equivalent to `ClampToEdge` for the
    /// non-tiling case (UVs that never leave `[0,1]`), so this is
    /// strictly the more general default, not a trade-off.
    pub fn create_sampler(&self) -> Sampler {
        let raw = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::Repeat,
            address_mode_w: wgpu::AddressMode::Repeat,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        Sampler { raw }
    }

    /// [`create_sampler`](Self::create_sampler)'s `ClampToEdge`
    /// counterpart, for full-screen post-process passes (blur/composite)
    /// rather than tiled surface UVs. A blur kernel deliberately samples
    /// *beyond* `[0,1]` near the screen edges (offset by several texels
    /// in each direction) — with `Repeat` addressing those out-of-range
    /// samples wrap around to the opposite edge of the texture, bleeding
    /// a shape's bloom halo across to the far side of the screen (the
    /// concrete bug this was added to fix: a glowing shape near the left
    /// edge showing a ghost of its glow on the right). `ClampToEdge`
    /// instead repeats the nearest edge pixel, which fades toward
    /// whatever's actually at that edge.
    pub fn create_clamp_sampler(&self) -> Sampler {
        let raw = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            ..Default::default()
        });
        Sampler { raw }
    }

    /// A comparison sampler — the only kind WGSL's `textureSampleCompare`
    /// accepts (a plain [`create_sampler`](Self::create_sampler)/
    /// [`create_clamp_sampler`](Self::create_clamp_sampler) sampler
    /// can't be bound to a `sampler_comparison` binding; `wgpu` rejects
    /// the mismatch at bind-group-creation time). `LessEqual`: a
    /// fragment is lit when its light-space depth is less than or equal
    /// to the shadow map's stored depth at that texel — the standard
    /// shadow-mapping comparison (see `graphics-core::submission`'s
    /// shadow-lookup WGSL for the bias that keeps this from
    /// self-shadowing every lit surface, "shadow acne").
    pub fn create_comparison_sampler(&self) -> Sampler {
        let raw = self.device.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Nearest,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..Default::default()
        });
        Sampler { raw }
    }
}

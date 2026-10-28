//! Texture and sampler creation on the device.

use crate::{Device, Sampler, Texture};

impl Device {
    /// A 2D texture in `format`, with `usage` flags the caller picks
    /// (depth/stencil formats support a narrower usage set than color
    /// formats on some backends, so this doesn't guess). `rendering-wgpu`'s
    /// `DepthTexture` is a thin wrapper calling this with `Depth32Float` +
    /// `RENDER_ATTACHMENT`; a color texture at `Rgba8Unorm` +
    /// `TEXTURE_BINDING | COPY_DST | COPY_SRC` is the other common case.
    #[must_use]
    pub fn create_texture(
        &self,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> Texture {
        self.create_mipped_texture([width, height], format, usage, 1)
    }

    /// [`create_texture`](Self::create_texture) with `mip_levels` levels
    /// (at least one): level `i` is `width >> i` by `height >> i`, each at
    /// least one texel, filled with
    /// [`write_texture_level`](Self::write_texture_level). The texture's view
    /// covers every level.
    #[must_use]
    pub fn create_mipped_texture(
        &self,
        [width, height]: [u32; 2],
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        mip_levels: u32,
    ) -> Texture {
        let raw = self.raw.create_texture(&wgpu::TextureDescriptor {
            label: None,
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: mip_levels.max(1),
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

    /// Uploads `data` (tightly packed, `bytes_per_texel` bytes per texel) to
    /// mip level `level` of `texture`: the level's whole extent, which is
    /// `width >> level` by `height >> level`, each at least one texel.
    pub fn write_texture_level(
        &self,
        texture: &Texture,
        level: u32,
        data: &[u8],
        bytes_per_texel: u32,
    ) {
        let width = (texture.width >> level).max(1);
        let height = (texture.height >> level).max(1);
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture.raw,
                mip_level: level,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(width * bytes_per_texel),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
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
    #[must_use]
    pub fn create_sampler(&self) -> Sampler {
        let raw = self.raw.create_sampler(&wgpu::SamplerDescriptor {
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
    #[must_use]
    pub fn create_clamp_sampler(&self) -> Sampler {
        let raw = self.raw.create_sampler(&wgpu::SamplerDescriptor {
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

    /// A sampler for an equirectangular environment with a mip chain:
    /// linear within and between levels, wrapping around the horizon (`u`) and
    /// clamping at the poles (`v`).
    #[must_use]
    pub fn create_environment_sampler(&self) -> Sampler {
        let raw = self.raw.create_sampler(&wgpu::SamplerDescriptor {
            label: None,
            address_mode_u: wgpu::AddressMode::Repeat,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::MipmapFilterMode::Linear,
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
    /// shadow-mapping comparison (see `rendering-domain::submission`'s
    /// shadow-lookup WGSL for the bias that keeps this from
    /// self-shadowing every lit surface, "shadow acne").
    #[must_use]
    pub fn create_comparison_sampler(&self) -> Sampler {
        let raw = self.raw.create_sampler(&wgpu::SamplerDescriptor {
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

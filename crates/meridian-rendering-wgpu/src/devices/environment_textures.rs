//! Environment texture creation on the device.

use meridian_rendering_domain::EnvironmentLevel;

use super::half_float::to_half;
use crate::{Device, Sampler, Texture};

/// Format of every environment texture: filterable and wide enough for sun-bright texels.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

/// The little-endian `Rgba16Float` bytes of `texels`, alpha `1.0`.
fn half_bytes(texels: &[[f32; 3]]) -> Vec<u8> {
    let mut bytes = Vec::with_capacity(texels.len() * 8);
    for texel in texels {
        for channel in [texel[0], texel[1], texel[2], 1.0] {
            bytes.extend_from_slice(&to_half(channel).to_le_bytes());
        }
    }
    bytes
}

impl Device {
    /// An `Rgba16Float` texture holding `levels` as its mip chain (sharpest
    /// first); level `i` must be `width >> i` by `height >> i` of the first, as
    /// `EnvironmentMap` guarantees.
    #[must_use]
    pub fn create_environment_texture(&self, levels: &[EnvironmentLevel]) -> Texture {
        let first = &levels[0];
        let texture = self.0.create_mipped_texture(
            [first.width, first.height],
            FORMAT,
            wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            u32::try_from(levels.len()).unwrap_or(u32::MAX),
        );
        for (index, level) in (0u32..).zip(levels) {
            self.0
                .write_texture_level(&texture, index, &half_bytes(&level.texels), 8);
        }
        texture
    }

    /// The sampler environment textures are read through.
    #[must_use]
    pub fn create_environment_sampler(&self) -> Sampler {
        self.0.create_environment_sampler()
    }

    /// A 1x1 black stand-in bound while no environment is set.
    #[must_use]
    pub fn create_neutral_environment_texture(&self) -> Texture {
        let level = EnvironmentLevel {
            width: 1,
            height: 1,
            texels: vec![[0.0; 3]],
        };
        self.create_environment_texture(&[level])
    }
}

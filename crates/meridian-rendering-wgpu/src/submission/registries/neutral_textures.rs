//! The 1x1 textures bound where a material names no map.

use crate::{Device, Texture};

/// Stand-ins that leave a material's factors as they are: every lit textured
/// draw binds all five maps, so a missing one gets the neutral value of its
/// slot.
#[derive(Debug)]
pub(crate) struct NeutralTextures {
    /// Opaque white colour: the albedo of a draw with only other maps, and a
    /// missing emissive map.
    pub white_albedo: Texture,
    /// Opaque white linear data: a missing metallic-roughness or occlusion map.
    pub white_linear: Texture,
    /// The flat tangent-space normal `(0, 0, 1)`: a missing normal map.
    pub flat_normal: Texture,
}

impl NeutralTextures {
    pub(crate) fn new(device: &Device) -> Self {
        let colour = |rgba: [u8; 4]| {
            let texture = device.create_texture_2d(1, 1);
            device.write_texture(&texture, &rgba);
            texture
        };
        let linear = |rgba: [u8; 4]| {
            let texture = device.create_linear_texture_2d(1, 1);
            device.write_texture(&texture, &rgba);
            texture
        };
        Self {
            white_albedo: colour([255; 4]),
            white_linear: linear([255; 4]),
            flat_normal: linear([128, 128, 255, 255]),
        }
    }
}

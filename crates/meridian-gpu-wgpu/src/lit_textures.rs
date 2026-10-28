//! The textures a lit, textured pipeline samples.

use crate::{SampledTexture, Texture};

/// What a textured, shadow-receiving lit shader reads besides its uniform:
/// the albedo and metallic-roughness maps (both read through the albedo's
/// sampler) and the shadow map with its comparison sampler.
#[derive(Debug, Clone, Copy)]
pub struct LitTextures<'a> {
    /// The base colour map and its sampler.
    pub albedo: SampledTexture<'a>,
    /// The metallic-roughness map (roughness in green, metalness in blue).
    pub metallic_roughness: &'a Texture,
    /// The shadow map and its comparison sampler.
    pub shadow: SampledTexture<'a>,
}

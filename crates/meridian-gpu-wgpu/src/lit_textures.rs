//! The textures a lit, textured pipeline samples.

use crate::{SampledTexture, SurfaceTextures};

/// What a textured, shadow-receiving lit shader reads besides its uniform:
/// the albedo map (and the sampler every material map is read through), the
/// surface maps, the shadow map with its comparison sampler and the
/// environment.
#[derive(Debug, Clone, Copy)]
pub struct LitTextures<'a> {
    /// The base colour map and its sampler.
    pub albedo: SampledTexture<'a>,
    /// The metallic-roughness, normal, occlusion and emissive maps.
    pub surface: SurfaceTextures<'a>,
    /// The shadow map and its comparison sampler.
    pub shadow: SampledTexture<'a>,
    /// The prefiltered environment (with its mip chain) and its sampler.
    pub environment: SampledTexture<'a>,
}

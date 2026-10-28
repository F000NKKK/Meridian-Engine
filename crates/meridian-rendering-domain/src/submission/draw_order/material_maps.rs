//! The textures a material names.

use crate::TextureHandle;

/// Every texture slot of a material that resolved against the texture
/// registry; `None` where the material names none or the texture is not
/// registered. Part of a [`BatchKey`](super::BatchKey): two draws share a call
/// only when they bind the same textures.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MaterialMaps {
    /// The base colour map; `None` draws flat colour (or white under other maps).
    pub albedo: Option<TextureHandle>,
    /// Roughness in green, metalness in blue (ADR 032).
    pub metallic_roughness: Option<TextureHandle>,
    /// A tangent-space normal map (ADR 033).
    pub normal: Option<TextureHandle>,
    /// Ambient occlusion in the red channel (ADR 033).
    pub occlusion: Option<TextureHandle>,
    /// The emitted colour map (ADR 033).
    pub emissive: Option<TextureHandle>,
}

impl MaterialMaps {
    /// Whether any slot names a texture, so the draw needs a textured pipeline.
    #[must_use]
    pub fn any(&self) -> bool {
        [
            self.albedo,
            self.metallic_roughness,
            self.normal,
            self.occlusion,
            self.emissive,
        ]
        .iter()
        .any(Option::is_some)
    }
}

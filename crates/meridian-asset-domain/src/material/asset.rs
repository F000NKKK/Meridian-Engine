//! [`MaterialAsset`]: a material as its file holds it.

use super::document::MaterialDocument;
use super::{MaterialBlend, MaterialError, read};
use crate::AssetKey;

/// A material file's contents: shading factors and the textures they
/// multiply, as asset references. Colours are linear RGB, not sRGB. Pure
/// data: no texture is loaded and no handle exists here.
#[derive(Debug, Clone, PartialEq)]
pub struct MaterialAsset {
    /// Linear RGBA multiplier of the albedo.
    pub base_color: [f32; 4],
    /// Metalness in `[0, 1]`.
    pub metallic: f32,
    /// Roughness in `[0, 1]`.
    pub roughness: f32,
    /// Linear RGB light the surface emits.
    pub emissive: [f32; 3],
    /// How strongly the normal map bends the surface.
    pub normal_scale: f32,
    /// How strongly occlusion dims ambient light.
    pub occlusion_strength: f32,
    /// How the surface is composited.
    pub blend: MaterialBlend,
    /// Skips lighting.
    pub unlit: bool,
    /// The colour texture (sRGB data).
    pub albedo: Option<AssetKey>,
    /// Roughness in green, metalness in blue (linear data).
    pub metallic_roughness: Option<AssetKey>,
    /// A tangent-space normal map (linear data).
    pub normal: Option<AssetKey>,
    /// Occlusion in red (linear data).
    pub occlusion: Option<AssetKey>,
    /// An emitted colour map (sRGB data).
    pub emissive_map: Option<AssetKey>,
}

impl Default for MaterialAsset {
    /// A mid-light grey dielectric of medium roughness: what a material
    /// file that says nothing means.
    fn default() -> Self {
        Self {
            base_color: [0.8, 0.8, 0.8, 1.0],
            metallic: 0.0,
            roughness: 0.5,
            emissive: [0.0; 3],
            normal_scale: 1.0,
            occlusion_strength: 1.0,
            blend: MaterialBlend::Opaque,
            unlit: false,
            albedo: None,
            metallic_roughness: None,
            normal: None,
            occlusion: None,
            emissive_map: None,
        }
    }
}

impl MaterialAsset {
    /// Reads a `.mmat` file's `bytes`: the binary form when they start with
    /// `SDAT`, UTF-8 text otherwise (never decided by the extension).
    ///
    /// # Errors
    ///
    /// [`MaterialError`] for bytes that do not decode, a `format` this does
    /// not read, and fields that are unknown or of the wrong type.
    pub fn read(bytes: &[u8]) -> Result<Self, MaterialError> {
        read::read(bytes)
    }

    /// The text form, as an editor saves it (always the newest `format`).
    #[must_use]
    pub fn to_text(&self) -> String {
        substrate_data::to_text(&MaterialDocument::of(self))
    }

    /// The `SDAT` binary form, for cooked content.
    #[must_use]
    pub fn to_bytes(&self) -> Vec<u8> {
        substrate_data::to_bytes(&MaterialDocument::of(self))
    }

    /// Every texture the material names, in a fixed order.
    pub fn textures(&self) -> impl Iterator<Item = &AssetKey> {
        [
            &self.albedo,
            &self.metallic_roughness,
            &self.normal,
            &self.occlusion,
            &self.emissive_map,
        ]
        .into_iter()
        .flatten()
    }
}

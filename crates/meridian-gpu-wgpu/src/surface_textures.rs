//! The per-material surface maps of a lit, textured pipeline.

use crate::Texture;

/// The maps that shape a surface beyond its base colour. All four are bound
/// even when a material has none (the renderer binds a neutral 1x1 texture),
/// so the pipeline layout does not depend on the material.
#[derive(Debug, Clone, Copy)]
pub struct SurfaceTextures<'a> {
    /// Roughness in green, metalness in blue.
    pub metallic_roughness: &'a Texture,
    /// A tangent-space normal map.
    pub normal: &'a Texture,
    /// Ambient occlusion in red.
    pub occlusion: &'a Texture,
    /// The emitted colour.
    pub emissive: &'a Texture,
}

//! The surface of one model part.

/// The parts of a PBR metallic-roughness material the renderer uses today
/// (ADRs 032 and 033).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ModelMaterial {
    /// Linear RGBA multiplier of the base colour (white and opaque when the
    /// source names none).
    pub base_color: [f32; 4],
    /// Index into [`ModelAsset::images`](crate::ModelAsset::images) of the
    /// base colour texture.
    pub base_color_image: Option<usize>,
    /// Metalness factor in `[0, 1]` (glTF's default is 1).
    pub metallic: f32,
    /// Roughness factor in `[0, 1]` (glTF's default is 1).
    pub roughness: f32,
    /// Index into [`ModelAsset::images`](crate::ModelAsset::images) of the
    /// metallic-roughness texture (roughness in green, metalness in blue;
    /// linear data).
    pub metallic_roughness_image: Option<usize>,
    /// Index into the images of the tangent-space normal map (linear data).
    pub normal_image: Option<usize>,
    /// How strongly the normal map bends the surface (glTF's `scale`, 1 by default).
    pub normal_scale: f32,
    /// Index into the images of the occlusion map (red channel, linear data).
    pub occlusion_image: Option<usize>,
    /// How strongly occlusion dims ambient light (glTF's `strength`, 1 by default).
    pub occlusion_strength: f32,
    /// Linear RGB emitted light; black when the source names none.
    pub emissive: [f32; 3],
    /// Index into the images of the emissive colour map (sRGB).
    pub emissive_image: Option<usize>,
}

impl Default for ModelMaterial {
    fn default() -> Self {
        Self {
            base_color: [1.0; 4],
            base_color_image: None,
            metallic: 1.0,
            roughness: 1.0,
            metallic_roughness_image: None,
            normal_image: None,
            normal_scale: 1.0,
            occlusion_image: None,
            occlusion_strength: 1.0,
            emissive: [0.0; 3],
            emissive_image: None,
        }
    }
}

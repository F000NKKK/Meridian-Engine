//! The surface of one model part.

/// The parts of a PBR metallic-roughness material the renderer uses today
/// (ADR 032).
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
}

impl Default for ModelMaterial {
    fn default() -> Self {
        Self {
            base_color: [1.0; 4],
            base_color_image: None,
            metallic: 1.0,
            roughness: 1.0,
            metallic_roughness_image: None,
        }
    }
}

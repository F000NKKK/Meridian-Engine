//! A glTF material as the renderer uses it.

use gltf::Material;
use gltf::material::{NormalTexture, OcclusionTexture};
use meridian_asset_domain::ModelMaterial;

/// The factors and textures of `material`: base colour, metalness and
/// roughness, normal, occlusion and emissive.
pub(crate) fn read(material: &Material<'_>) -> ModelMaterial {
    let pbr = material.pbr_metallic_roughness();
    let normal = material.normal_texture();
    let occlusion = material.occlusion_texture();
    ModelMaterial {
        base_color: pbr.base_color_factor(),
        base_color_image: pbr
            .base_color_texture()
            .map(|info| info.texture().source().index()),
        metallic: pbr.metallic_factor(),
        roughness: pbr.roughness_factor(),
        metallic_roughness_image: pbr
            .metallic_roughness_texture()
            .map(|info| info.texture().source().index()),
        normal_image: normal.as_ref().map(|info| info.texture().source().index()),
        normal_scale: normal.as_ref().map_or(1.0, NormalTexture::scale),
        occlusion_image: occlusion
            .as_ref()
            .map(|info| info.texture().source().index()),
        occlusion_strength: occlusion.as_ref().map_or(1.0, OcclusionTexture::strength),
        emissive: material.emissive_factor(),
        emissive_image: material
            .emissive_texture()
            .map(|info| info.texture().source().index()),
    }
}

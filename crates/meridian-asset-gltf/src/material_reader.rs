//! A glTF material as the renderer uses it.

use gltf::Material;
use meridian_asset_domain::ModelMaterial;

/// The base colour factor and base colour texture of `material`. Other
/// slots (metallic-roughness, normal, occlusion, emissive) are ignored.
pub(crate) fn read(material: &Material<'_>) -> ModelMaterial {
    let pbr = material.pbr_metallic_roughness();
    ModelMaterial {
        base_color: pbr.base_color_factor(),
        base_color_image: pbr
            .base_color_texture()
            .map(|info| info.texture().source().index()),
    }
}

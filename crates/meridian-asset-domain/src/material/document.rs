//! The `.mmat` document as Substrate Data reads and writes it.

use substrate_data::{Id, Schema};

use super::{MATERIAL_FORMAT, MaterialAsset, MaterialBlend};
use crate::{AssetKey, AssetPathKind};

/// A `.mmat` file's fields, strictly: an unknown key is a problem, so a
/// file from a newer version is refused instead of losing data. Every field
/// but `format` falls back to [`MaterialAsset::default`].
#[derive(Schema)]
pub(super) struct MaterialDocument {
    pub(super) format: u32,
    #[data(default = "base_color")]
    base_color: [f32; 4],
    #[data(default = "metallic")]
    metallic: f32,
    #[data(default = "roughness")]
    roughness: f32,
    #[data(default)]
    emissive: [f32; 3],
    #[data(default = "one")]
    normal_scale: f32,
    #[data(default = "one")]
    occlusion_strength: f32,
    #[data(default)]
    blend: MaterialBlend,
    #[data(default)]
    unlit: bool,
    #[data(default)]
    albedo: Option<Id<AssetPathKind>>,
    #[data(default)]
    metallic_roughness: Option<Id<AssetPathKind>>,
    #[data(default)]
    normal: Option<Id<AssetPathKind>>,
    #[data(default)]
    occlusion: Option<Id<AssetPathKind>>,
    #[data(default)]
    emissive_map: Option<Id<AssetPathKind>>,
}

fn base_color() -> [f32; 4] {
    MaterialAsset::default().base_color
}

fn metallic() -> f32 {
    MaterialAsset::default().metallic
}

fn roughness() -> f32 {
    MaterialAsset::default().roughness
}

fn one() -> f32 {
    1.0
}

fn reference(key: &Option<AssetKey>) -> Option<Id<AssetPathKind>> {
    key.clone().map(Id::new)
}

impl MaterialDocument {
    /// The document of `asset`, as written: every field present.
    pub(super) fn of(asset: &MaterialAsset) -> Self {
        MaterialDocument {
            format: MATERIAL_FORMAT,
            base_color: asset.base_color,
            metallic: asset.metallic,
            roughness: asset.roughness,
            emissive: asset.emissive,
            normal_scale: asset.normal_scale,
            occlusion_strength: asset.occlusion_strength,
            blend: asset.blend,
            unlit: asset.unlit,
            albedo: reference(&asset.albedo),
            metallic_roughness: reference(&asset.metallic_roughness),
            normal: reference(&asset.normal),
            occlusion: reference(&asset.occlusion),
            emissive_map: reference(&asset.emissive_map),
        }
    }

    /// The asset this document describes.
    pub(super) fn into_asset(self) -> MaterialAsset {
        let key = |id: Option<Id<AssetPathKind>>| id.map(|id| id.id);
        MaterialAsset {
            base_color: self.base_color,
            metallic: self.metallic,
            roughness: self.roughness,
            emissive: self.emissive,
            normal_scale: self.normal_scale,
            occlusion_strength: self.occlusion_strength,
            blend: self.blend,
            unlit: self.unlit,
            albedo: key(self.albedo),
            metallic_roughness: key(self.metallic_roughness),
            normal: key(self.normal),
            occlusion: key(self.occlusion),
            emissive_map: key(self.emissive_map),
        }
    }
}

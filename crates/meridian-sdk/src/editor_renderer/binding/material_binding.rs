//! A `.mmat` material turned into the runtime [`Material`].

use meridian_asset_domain::{AssetKey, MaterialAsset, MaterialBlend};
use meridian_rendering_domain::{BlendMode, Material};

use super::{AssetBinder, TextureSlot};
use crate::GraphicsSession;

/// The runtime material of `asset`, its textures bound through `binder`.
/// The textures `binder` has no answer for are returned and left off the
/// material, which then draws with its factors alone.
pub fn bind_material(
    session: &mut GraphicsSession,
    asset: &MaterialAsset,
    binder: &mut dyn AssetBinder,
) -> (Material, Vec<AssetKey>) {
    let mut missing = Vec::new();
    let mut texture = |key: &Option<AssetKey>, slot: TextureSlot| {
        let key = key.as_ref()?;
        let handle = binder.texture(session, key, slot);
        if handle.is_none() {
            missing.push(key.clone());
        }
        handle
    };
    let material = Material {
        albedo: texture(&asset.albedo, TextureSlot::Albedo),
        metallic_roughness: texture(&asset.metallic_roughness, TextureSlot::MetallicRoughness),
        normal: texture(&asset.normal, TextureSlot::Normal),
        occlusion: texture(&asset.occlusion, TextureSlot::Occlusion),
        emissive_map: texture(&asset.emissive_map, TextureSlot::Emissive),
        base_color_factor: asset.base_color,
        normal_scale: asset.normal_scale,
        occlusion_strength: asset.occlusion_strength,
        emissive: asset.emissive,
        roughness: asset.roughness,
        metallic: asset.metallic,
        unlit: asset.unlit,
        blend: match asset.blend {
            MaterialBlend::Opaque => BlendMode::Opaque,
            MaterialBlend::AlphaBlend => BlendMode::AlphaBlend,
        },
        ..Material::default()
    };
    (material, missing)
}

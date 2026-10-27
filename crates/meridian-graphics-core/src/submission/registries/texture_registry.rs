//! Registered textures by handle.

use meridian_graphics_driver::{Device, Texture};
use meridian_memory_core::ResourcePool;

use crate::TextureHandle;

/// GPU-resident textures, handle-addressed by [`TextureHandle`] — the
/// registry a [`Material::albedo`](crate::scene::Material::albedo) resolves against. Upload happens once
/// via [`upload`](Self::upload), from an already-decoded
/// `asset-core::ImageAsset` (RGBA8) — this bridge does not decode image
/// bytes itself, matching the crate's existing "decoding belongs to
/// `asset-core`" boundary.
#[derive(Debug, Default)]
pub struct TextureRegistry {
    textures: ResourcePool<Texture>,
}

impl TextureRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Uploads `image`'s pixels to a new GPU texture and returns its
    /// handle.
    pub fn upload(
        &mut self,
        device: &Device,
        image: &meridian_asset_core::ImageAsset,
    ) -> TextureHandle {
        let texture = device.create_texture_2d(image.width, image.height);
        device.write_texture(&texture, &image.pixels);
        TextureHandle::new(self.textures.insert(texture))
    }

    /// Uploads a 1×1 texture of one `rgba` color and returns its handle.
    /// Sprites that are plain colored rectangles (UI backgrounds) point at
    /// a white one and carry their color in `Sprite::tint`.
    pub fn upload_solid(&mut self, device: &Device, rgba: [u8; 4]) -> TextureHandle {
        let texture = device.create_texture_2d(1, 1);
        device.write_texture(&texture, &rgba);
        TextureHandle::new(self.textures.insert(texture))
    }

    pub fn unregister(&mut self, handle: TextureHandle) -> Option<Texture> {
        self.textures.remove(handle.handle)
    }

    pub fn get(&self, handle: TextureHandle) -> Option<&Texture> {
        self.textures.get(handle.handle)
    }
}

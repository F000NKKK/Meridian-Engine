//! An in-memory [`TextureStore`](crate::TextureStore).

use meridian_asset_core::ImageAsset;
use meridian_memory_core::ResourcePool;

use crate::{TextureHandle, TextureStore};

/// Keeps each texture's size instead of pixels; no GPU.
#[derive(Debug, Default)]
pub struct FakeTextureStore {
    sizes: ResourcePool<[u32; 2]>,
}

impl FakeTextureStore {
    /// An empty store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The `[width, height]` of a live texture.
    #[must_use]
    pub fn size(&self, handle: TextureHandle) -> Option<[u32; 2]> {
        self.sizes.get(handle.handle).copied()
    }
}

impl TextureStore for FakeTextureStore {
    fn upload(&mut self, image: &ImageAsset) -> TextureHandle {
        TextureHandle::new(self.sizes.insert([image.width, image.height]))
    }

    fn upload_solid(&mut self, _rgba: [u8; 4]) -> TextureHandle {
        TextureHandle::new(self.sizes.insert([1, 1]))
    }

    fn contains(&self, handle: TextureHandle) -> bool {
        self.sizes.get(handle.handle).is_some()
    }

    fn release(&mut self, handle: TextureHandle) -> bool {
        self.sizes.remove(handle.handle).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::run_texture_store_contract;

    #[test]
    fn passes_the_texture_store_contract() {
        run_texture_store_contract(FakeTextureStore::new);
    }

    #[test]
    fn remembers_sizes() {
        let mut store = FakeTextureStore::new();
        let handle = store.upload(&ImageAsset {
            width: 2,
            height: 3,
            pixels: vec![0; 24],
        });
        assert_eq!(store.size(handle), Some([2, 3]));
        let solid = store.upload_solid([1, 2, 3, 4]);
        assert_eq!(store.size(solid), Some([1, 1]));
        assert!(store.release(solid));
        assert_eq!(store.size(solid), None);
    }
}

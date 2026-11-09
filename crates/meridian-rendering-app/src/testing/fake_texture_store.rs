//! An in-memory [`TextureStore`](crate::TextureStore).

use meridian_asset_domain::ImageAsset;
use meridian_memory::ResourcePool;
use meridian_rendering_domain::TextureHandle;

use crate::TextureStore;

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

    fn upload_linear(&mut self, image: &ImageAsset) -> TextureHandle {
        self.upload(image)
    }

    fn upload_solid(&mut self, _rgba: [u8; 4]) -> TextureHandle {
        TextureHandle::new(self.sizes.insert([1, 1]))
    }

    fn update(&mut self, handle: TextureHandle, image: &ImageAsset) -> bool {
        self.size(handle) == Some([image.width, image.height])
            && usize::try_from(image.width)
                .ok()
                .and_then(|width| {
                    usize::try_from(image.height)
                        .ok()
                        .and_then(|height| width.checked_mul(height))
                })
                .and_then(|pixels| pixels.checked_mul(4))
                == Some(image.pixels.len())
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
    #[test]
    fn updates_preserve_identity_and_reject_invalid_layouts() {
        let mut store = FakeTextureStore::new();
        let image = ImageAsset {
            width: 2,
            height: 2,
            pixels: vec![255; 16],
        };
        let handle = store.upload(&image);
        assert!(store.update(handle, &image));
        assert_eq!(store.size(handle), Some([2, 2]));
        assert!(!store.update(
            handle,
            &ImageAsset {
                width: 1,
                height: 4,
                pixels: vec![0; 16]
            }
        ));
        assert!(!store.update(
            handle,
            &ImageAsset {
                width: 2,
                height: 2,
                pixels: vec![0; 15]
            }
        ));
        assert_eq!(store.size(handle), Some([2, 2]));
        assert!(store.release(handle));
        assert!(!store.update(handle, &image));
        let replacement = store.upload(&image);
        assert_ne!(replacement, handle);
        assert!(!store.update(handle, &image));
        assert!(store.update(replacement, &image));
    }
}

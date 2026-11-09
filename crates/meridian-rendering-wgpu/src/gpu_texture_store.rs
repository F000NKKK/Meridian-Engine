//! GPU textures behind the [`TextureStore`] port.

use meridian_asset_domain::ImageAsset;
use meridian_rendering_app::TextureStore;
use meridian_rendering_domain::TextureHandle;

use crate::{Device, TextureRegistry};

/// The `wgpu` implementation of [`TextureStore`]: every upload creates a GPU
/// texture on the device and registers it under a [`TextureHandle`].
#[derive(Debug)]
pub struct GpuTextureStore {
    device: Device,
    registry: TextureRegistry,
}

impl GpuTextureStore {
    /// An empty store uploading to `device`.
    #[must_use]
    pub fn new(device: Device) -> Self {
        Self {
            device,
            registry: TextureRegistry::new(),
        }
    }

    /// The registry the renderer resolves a material's albedo against.
    #[must_use]
    pub fn registry(&self) -> &TextureRegistry {
        &self.registry
    }
}

impl TextureStore for GpuTextureStore {
    fn upload(&mut self, image: &ImageAsset) -> TextureHandle {
        self.registry.upload(&self.device, image)
    }

    fn upload_linear(&mut self, image: &ImageAsset) -> TextureHandle {
        self.registry.upload_linear(&self.device, image)
    }

    fn upload_solid(&mut self, rgba: [u8; 4]) -> TextureHandle {
        self.registry.upload_solid(&self.device, rgba)
    }

    fn update(&mut self, handle: TextureHandle, image: &ImageAsset) -> bool {
        self.registry.update(&self.device, handle, image)
    }

    fn contains(&self, handle: TextureHandle) -> bool {
        self.registry.get(handle).is_some()
    }

    fn release(&mut self, handle: TextureHandle) -> bool {
        self.registry.unregister(handle).is_some()
    }
}

#[cfg(all(test, feature = "hw-tests"))]
mod tests {
    use meridian_rendering_app::testing::run_texture_store_contract;

    use super::*;

    #[tokio::test]
    async fn passes_the_texture_store_contract_on_a_real_device() {
        let Ok(device) = Device::new().await else {
            eprintln!("skipping: no GPU device available");
            return;
        };
        run_texture_store_contract(|| GpuTextureStore::new(device.clone()));
    }
    #[tokio::test]
    async fn updates_keep_live_handles_and_reject_stale_or_resized_images() {
        let device = Device::new()
            .await
            .expect("GPU texture update test requires an adapter");
        let mut store = GpuTextureStore::new(device);
        let image = ImageAsset {
            width: 2,
            height: 2,
            pixels: vec![255; 16],
        };
        let handle = store.upload(&image);
        assert!(store.update(handle, &image));
        assert!(store.contains(handle));
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
        assert!(store.release(handle));
        assert!(!store.update(handle, &image));
        let new_handle = store.upload(&image);
        assert_ne!(handle, new_handle);
        assert!(!store.update(handle, &image));
        assert!(store.update(new_handle, &image));
    }
}

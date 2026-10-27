//! GPU textures behind the [`TextureStore`] port.

use meridian_asset_core::ImageAsset;
use meridian_graphics_core::TextureHandle;
use meridian_rendering_app::TextureStore;

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

    fn upload_solid(&mut self, rgba: [u8; 4]) -> TextureHandle {
        self.registry.upload_solid(&self.device, rgba)
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
}

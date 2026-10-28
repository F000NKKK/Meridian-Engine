//! The port that puts decoded images on the GPU.

use meridian_asset_domain::ImageAsset;
use meridian_rendering_domain::TextureHandle;

/// Where decoded images become sampleable textures, addressed by
/// [`TextureHandle`](meridian_rendering_domain::TextureHandle).
///
/// Uploading is the adapter's work (it owns the device); callers hold only
/// handles (ADR 002). `rendering-wgpu` implements it with GPU textures;
/// [`testing::FakeTextureStore`](crate::testing::FakeTextureStore) is the
/// in-memory fake and
/// [`testing::run_texture_store_contract`](crate::testing::run_texture_store_contract)
/// the suite every implementation passes.
pub trait TextureStore {
    /// Uploads `image`'s RGBA8 pixels and returns the new texture's handle.
    fn upload(&mut self, image: &ImageAsset) -> TextureHandle;

    /// Uploads `image`'s RGBA8 pixels as linear data, with no sRGB decode on
    /// sampling: the layout for maps that hold numbers, not colours, such as
    /// a metallic-roughness map (ADR 031).
    fn upload_linear(&mut self, image: &ImageAsset) -> TextureHandle;

    /// Uploads a 1x1 texture of one `rgba` colour: what plain coloured
    /// rectangles point at, carrying their colour in a tint.
    fn upload_solid(&mut self, rgba: [u8; 4]) -> TextureHandle;

    /// Whether `handle` names a live texture of this store.
    fn contains(&self, handle: TextureHandle) -> bool;

    /// Frees the texture; `false` when `handle` was not live.
    fn release(&mut self, handle: TextureHandle) -> bool;
}

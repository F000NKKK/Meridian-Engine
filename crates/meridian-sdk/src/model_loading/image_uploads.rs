//! A model's images as uploaded textures.

use std::collections::HashMap;

use meridian_asset_domain::{Decoder, EncodedImage, ImageAsset, ModelMaterial};
use meridian_rendering_app::TextureStore;
use meridian_rendering_domain::{Material, TextureHandle};

use crate::any_image_decoder::AnyImageDecoder;

/// The model's images as uploaded textures, each at most once per way of
/// reading it: colour data (base colour, emissive) or linear data (everything
/// else), since one image can serve both.
pub(super) struct ImageUploads<'a, S> {
    images: &'a [EncodedImage],
    textures: &'a mut S,
    colour: HashMap<usize, Option<TextureHandle>>,
    linear: HashMap<usize, Option<TextureHandle>>,
}

impl<'a, S: TextureStore> ImageUploads<'a, S> {
    pub(super) fn new(images: &'a [EncodedImage], textures: &'a mut S) -> Self {
        Self {
            images,
            textures,
            colour: HashMap::new(),
            linear: HashMap::new(),
        }
    }

    fn colour(&mut self, index: Option<usize>) -> Option<TextureHandle> {
        let index = index?;
        let (images, textures) = (self.images, &mut *self.textures);
        *self
            .colour
            .entry(index)
            .or_insert_with(|| upload(&images[index].bytes, index, |image| textures.upload(image)))
    }

    fn linear(&mut self, index: Option<usize>) -> Option<TextureHandle> {
        let index = index?;
        let (images, textures) = (self.images, &mut *self.textures);
        *self.linear.entry(index).or_insert_with(|| {
            upload(&images[index].bytes, index, |image| {
                textures.upload_linear(image)
            })
        })
    }

    /// The renderer's material for `source`, with its textures uploaded.
    pub(super) fn material(&mut self, source: &ModelMaterial) -> Material {
        Material {
            albedo: self.colour(source.base_color_image),
            metallic_roughness: self.linear(source.metallic_roughness_image),
            normal: self.linear(source.normal_image),
            occlusion: self.linear(source.occlusion_image),
            emissive_map: self.colour(source.emissive_image),
            base_color_factor: source.base_color,
            metallic: source.metallic,
            roughness: source.roughness,
            normal_scale: source.normal_scale,
            occlusion_strength: source.occlusion_strength,
            emissive: source.emissive,
            ..Material::default()
        }
    }
}

/// Decodes `bytes` and hands the image to `put`; a failure is logged and
/// leaves the part without that texture.
fn upload(
    bytes: &[u8],
    index: usize,
    put: impl FnOnce(&ImageAsset) -> TextureHandle,
) -> Option<TextureHandle> {
    match AnyImageDecoder.decode(bytes) {
        Ok(image) => Some(put(&image)),
        Err(err) => {
            meridian_diagnostics::log_warn!("model image {index} not used: {err}");
            None
        }
    }
}

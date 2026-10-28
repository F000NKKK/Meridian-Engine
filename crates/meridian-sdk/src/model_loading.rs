//! A decoded glTF model registered with the GPU ports: one mesh and one
//! material per part (ADR 030).

use std::collections::HashMap;

use meridian_asset_domain::{Decoder, EncodedImage, ImageAsset, ModelAsset, ModelMaterial};
use meridian_rendering_app::{MeshStore, TextureStore};
use meridian_rendering_domain::{Material, MeshHandle, MeshSource, TextureHandle, smooth_normals};

use crate::any_image_decoder::AnyImageDecoder;

/// One drawable piece of a loaded model: the mesh to draw and the material
/// to draw it with. Its geometry is already in the model's own space, so a
/// renderable places it with the frame of the whole model.
#[derive(Debug, Clone, Copy)]
pub struct ModelPiece {
    /// The registered geometry.
    pub mesh: MeshHandle,
    /// The surface, with its texture already uploaded.
    pub material: Material,
}

/// Registers every part of `model`: meshes with `meshes` (missing normals
/// are derived from the triangles, missing UVs are zero), images with
/// `textures` once each per use (colour or linear), however many parts use them. An image the engine
/// cannot decode (an unknown format, or a corrupt file) is logged and its parts keep their base
/// colour only.
pub(crate) fn register(
    model: ModelAsset,
    meshes: &mut impl MeshStore,
    textures: &mut impl TextureStore,
) -> Result<Vec<ModelPiece>, String> {
    let mut uploads = ImageUploads::new(&model.images, textures);
    let mut pieces = Vec::with_capacity(model.parts.len());
    for part in model.parts {
        let material = uploads.material(&part.material);
        let normals = if part.normals.is_empty() {
            smooth_normals(&part.positions, &part.indices)
        } else {
            part.normals
        };
        let uvs = if part.uvs.is_empty() {
            vec![[0.0, 0.0]; part.positions.len()]
        } else {
            part.uvs
        };
        let mesh = meshes
            .add(MeshSource {
                positions: part.positions,
                normals,
                uvs,
                indices: part.indices,
            })
            .map_err(|err| err.to_string())?;
        pieces.push(ModelPiece { mesh, material });
    }
    Ok(pieces)
}

/// The model's images as uploaded textures, each at most once per way of
/// reading it: colour data (base colour, emissive) or linear data (everything
/// else), since one image can serve both.
struct ImageUploads<'a, S> {
    images: &'a [EncodedImage],
    textures: &'a mut S,
    colour: HashMap<usize, Option<TextureHandle>>,
    linear: HashMap<usize, Option<TextureHandle>>,
}

impl<'a, S: TextureStore> ImageUploads<'a, S> {
    fn new(images: &'a [EncodedImage], textures: &'a mut S) -> Self {
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
    fn material(&mut self, source: &ModelMaterial) -> Material {
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

#[cfg(test)]
mod tests {
    use meridian_asset_domain::ModelPart;
    use meridian_rendering_app::testing::FakeTextureStore;
    use meridian_rendering_domain::MeshRegistry;

    use super::*;

    fn triangle(material: ModelMaterial) -> ModelPart {
        ModelPart {
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]],
            indices: vec![0, 1, 2],
            material,
            ..ModelPart::default()
        }
    }

    fn png_1x1() -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, 1, 1);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[255, 0, 0, 255]).unwrap();
        writer.finish().unwrap();
        bytes
    }

    fn textured(index: usize) -> ModelMaterial {
        ModelMaterial {
            base_color: [0.5, 0.5, 0.5, 1.0],
            base_color_image: Some(index),
            metallic: 0.25,
            roughness: 0.75,
            ..ModelMaterial::default()
        }
    }

    #[test]
    fn missing_normals_and_uvs_are_filled_in() {
        let model = ModelAsset {
            parts: vec![triangle(ModelMaterial::default())],
            images: Vec::new(),
            ..ModelAsset::default()
        };
        let mut meshes = MeshRegistry::new();
        let pieces = register(model, &mut meshes, &mut FakeTextureStore::new()).unwrap();
        let source = meshes.get(pieces[0].mesh).unwrap();
        assert_eq!(source.normals.len(), 3);
        assert_eq!(source.uvs, vec![[0.0, 0.0]; 3]);
        assert_eq!(pieces[0].material.albedo, None);
    }

    #[test]
    fn parts_sharing_an_image_share_one_texture() {
        let model = ModelAsset {
            parts: vec![triangle(textured(0)), triangle(textured(0))],
            images: vec![EncodedImage { bytes: png_1x1() }],
            ..ModelAsset::default()
        };
        let pieces = register(
            model,
            &mut MeshRegistry::new(),
            &mut FakeTextureStore::new(),
        )
        .unwrap();
        assert!(pieces[0].material.albedo.is_some());
        assert_eq!(pieces[0].material.albedo, pieces[1].material.albedo);
        assert_eq!(pieces[0].material.base_color_factor, [0.5, 0.5, 0.5, 1.0]);
        assert_eq!(
            (pieces[0].material.metallic, pieces[0].material.roughness),
            (0.25, 0.75)
        );
    }

    #[test]
    fn a_metallic_roughness_map_is_its_own_linear_texture() {
        let material = ModelMaterial {
            metallic_roughness_image: Some(0),
            ..textured(0)
        };
        let model = ModelAsset {
            parts: vec![triangle(material), triangle(material)],
            images: vec![EncodedImage { bytes: png_1x1() }],
            ..ModelAsset::default()
        };
        let pieces = register(
            model,
            &mut MeshRegistry::new(),
            &mut FakeTextureStore::new(),
        )
        .unwrap();
        let (first, second) = (pieces[0].material, pieces[1].material);
        assert!(first.metallic_roughness.is_some());
        assert_ne!(
            first.albedo, first.metallic_roughness,
            "one image, read as colour and as linear data, is two textures"
        );
        assert_eq!(first.metallic_roughness, second.metallic_roughness);
    }

    #[test]
    fn normal_occlusion_and_emissive_maps_are_uploaded_by_how_they_are_read() {
        let material = ModelMaterial {
            normal_image: Some(0),
            normal_scale: 0.5,
            occlusion_image: Some(0),
            occlusion_strength: 0.25,
            emissive: [1.0, 0.5, 0.0],
            emissive_image: Some(0),
            ..ModelMaterial::default()
        };
        let model = ModelAsset {
            parts: vec![triangle(material)],
            images: vec![EncodedImage { bytes: png_1x1() }],
            ..ModelAsset::default()
        };
        let pieces = register(
            model,
            &mut MeshRegistry::new(),
            &mut FakeTextureStore::new(),
        )
        .unwrap();
        let made = pieces[0].material;
        assert_eq!(made.normal, made.occlusion, "both are linear data");
        assert_ne!(made.normal, made.emissive_map, "emissive is colour data");
        assert_eq!(
            (made.normal_scale, made.occlusion_strength, made.emissive),
            (0.5, 0.25, [1.0, 0.5, 0.0])
        );
    }

    #[test]
    fn an_undecodable_image_leaves_the_base_colour() {
        let model = ModelAsset {
            parts: vec![triangle(textured(0))],
            images: vec![EncodedImage {
                bytes: b"\xff\xd8\xff jpeg".to_vec(),
            }],
            ..ModelAsset::default()
        };
        let pieces = register(
            model,
            &mut MeshRegistry::new(),
            &mut FakeTextureStore::new(),
        )
        .unwrap();
        assert_eq!(pieces[0].material.albedo, None);
    }
}

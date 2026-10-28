//! A decoded glTF model registered with the GPU ports: one mesh and one
//! material per part (ADR 030).

use meridian_asset_domain::{Decoder, ModelAsset};
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
/// `textures` once each, however many parts use them. An image the engine
/// cannot decode (an unknown format, or a corrupt file) is logged and its parts keep their base
/// colour only.
pub(crate) fn register(
    model: ModelAsset,
    meshes: &mut impl MeshStore,
    textures: &mut impl TextureStore,
) -> Result<Vec<ModelPiece>, String> {
    let mut uploaded: Vec<Option<Option<TextureHandle>>> = vec![None; model.images.len()];
    let mut pieces = Vec::with_capacity(model.parts.len());
    for part in model.parts {
        let albedo = part.material.base_color_image.and_then(|index| {
            *uploaded[index]
                .get_or_insert_with(|| upload(&model.images[index].bytes, index, textures))
        });
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
        let material = Material {
            albedo,
            base_color_factor: part.material.base_color,
            ..Material::default()
        };
        pieces.push(ModelPiece { mesh, material });
    }
    Ok(pieces)
}

fn upload(bytes: &[u8], index: usize, textures: &mut impl TextureStore) -> Option<TextureHandle> {
    match AnyImageDecoder.decode(bytes) {
        Ok(image) => Some(textures.upload(&image)),
        Err(err) => {
            meridian_diagnostics::log_warn!("model image {index} not used: {err}");
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use meridian_asset_domain::{EncodedImage, ModelMaterial, ModelPart};
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
        }
    }

    #[test]
    fn missing_normals_and_uvs_are_filled_in() {
        let model = ModelAsset {
            parts: vec![triangle(ModelMaterial::default())],
            images: Vec::new(),
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
    }

    #[test]
    fn an_undecodable_image_leaves_the_base_colour() {
        let model = ModelAsset {
            parts: vec![triangle(textured(0))],
            images: vec![EncodedImage {
                bytes: b"\xff\xd8\xff jpeg".to_vec(),
            }],
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

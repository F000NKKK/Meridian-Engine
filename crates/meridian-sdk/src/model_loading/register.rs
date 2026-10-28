//! Registering a decoded model with the GPU ports.

use meridian_asset_domain::ModelAsset;
use meridian_rendering_app::{MeshStore, TextureStore};
use meridian_rendering_domain::{MeshMorph, MeshSource, MorphTarget, VertexSkin, smooth_normals};

use super::image_uploads::ImageUploads;
use super::loaded_model::LoadedModel;
use super::model_piece::ModelPiece;
use super::rig::ModelRig;

/// Registers every part of `model`: meshes with `meshes` (missing normals
/// are derived from the triangles, missing UVs are zero), images with
/// `textures` once each per use (colour or linear), however many parts use them. An image the engine
/// cannot decode (an unknown format, or a corrupt file) is logged and its parts keep their base
/// colour only.
pub(crate) fn register(
    model: ModelAsset,
    meshes: &mut impl MeshStore,
    textures: &mut impl TextureStore,
) -> Result<LoadedModel, String> {
    let rig = ModelRig::of(&model)?;
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
        let skeleton = part.skin.as_ref().map(|skin| skin.skeleton);
        let mesh = meshes
            .add(MeshSource {
                positions: part.positions,
                normals,
                uvs,
                indices: part.indices,
                skin: part.skin.map(|skin| VertexSkin {
                    joints: skin.joints,
                    weights: skin.weights,
                }),
                morph: part.morph.map(|morph| MeshMorph {
                    targets: morph
                        .targets
                        .into_iter()
                        .map(|target| MorphTarget {
                            positions: target.positions,
                            normals: target.normals,
                        })
                        .collect(),
                }),
            })
            .map_err(|err| err.to_string())?;
        pieces.push(ModelPiece {
            mesh,
            material,
            skeleton,
        });
    }
    Ok(LoadedModel { pieces, rig })
}

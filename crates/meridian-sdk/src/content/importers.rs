//! [`Importers`]: the importers the engine ships, registered by name.

use meridian_asset_app::{ImporterRegistry, RegisterError};
use meridian_asset_domain::{
    AudioClip, BmpDecoder, ImageAsset, ImporterName, ImporterNameError, MeshAsset, ModelAsset,
    ObjDecoder, ShaderSource, ShaderSourceDecoder, WavDecoder,
};
use meridian_asset_gltf::GltfDecoder;
use meridian_asset_jpeg::JpegDecoder;
use meridian_asset_png::PngDecoder;
use meridian_asset_symphonia::AnyAudioDecoder;
use thiserror::Error;

use crate::any_image_decoder::AnyImageDecoder;

/// One registry per kind of content, each holding the importers the engine
/// ships under the names a `.meta` file's `importer` uses. The composition
/// root builds it once and shares the registries (`Arc`) with the loaders;
/// an application adds its own importers to a registry before it is shared
/// ([`standard`](Self::standard) gives plain registries to extend).
///
/// | registry | content | importer names |
/// |---|---|---|
/// | `images` | [`ImageAsset`] | `image` (by magic), `bmp`, `png`, `jpeg` |
/// | `meshes` | [`MeshAsset`] | `obj` |
/// | `models` | [`ModelAsset`] | `gltf` |
/// | `shaders` | [`ShaderSource`] | `wgsl` |
/// | `audio` | [`AudioClip`] | `audio` (by magic), `wav` |
#[derive(Debug)]
pub struct Importers {
    /// Images.
    pub images: ImporterRegistry<ImageAsset>,
    /// Meshes.
    pub meshes: ImporterRegistry<MeshAsset>,
    /// Models with materials and textures.
    pub models: ImporterRegistry<ModelAsset>,
    /// Shader sources.
    pub shaders: ImporterRegistry<ShaderSource>,
    /// Decoded audio.
    pub audio: ImporterRegistry<AudioClip>,
}

impl Importers {
    /// The engine's importers.
    ///
    /// # Errors
    ///
    /// [`ImportersError`] if a shipped name were malformed or used
    /// twice (a bug in this table, caught by its test).
    pub fn standard() -> Result<Self, ImportersError> {
        let mut images = ImporterRegistry::new();
        images.register_decoder(name("image")?, AnyImageDecoder)?;
        images.register_decoder(name("bmp")?, BmpDecoder)?;
        images.register_decoder(name("png")?, PngDecoder)?;
        images.register_decoder(name("jpeg")?, JpegDecoder)?;
        let mut meshes = ImporterRegistry::new();
        meshes.register_decoder(name("obj")?, ObjDecoder)?;
        let mut models = ImporterRegistry::new();
        models.register_decoder(name("gltf")?, GltfDecoder)?;
        let mut shaders = ImporterRegistry::new();
        shaders.register_decoder(name("wgsl")?, ShaderSourceDecoder)?;
        let mut audio = ImporterRegistry::new();
        audio.register_decoder(name("audio")?, AnyAudioDecoder)?;
        audio.register_decoder(name("wav")?, WavDecoder)?;
        Ok(Importers {
            images,
            meshes,
            models,
            shaders,
            audio,
        })
    }
}

/// Why [`Importers::standard`] could not build its table.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[non_exhaustive]
pub enum ImportersError {
    /// A shipped importer name is not a valid [`ImporterName`].
    #[error(transparent)]
    Name(#[from] ImporterNameError),
    /// Two shipped importers share a name.
    #[error(transparent)]
    Register(#[from] RegisterError),
}

fn name(text: &str) -> Result<ImporterName, ImporterNameError> {
    ImporterName::new(text)
}

#[cfg(test)]
mod tests {
    use meridian_asset_domain::{AssetId, AssetMeta};

    use super::*;

    #[test]
    fn the_shipped_names_are_unique_and_resolve() {
        let importers = Importers::standard().expect("unique names");
        let obj = AssetMeta::new(AssetId::from_bits(1), name("obj").unwrap());
        let mesh = importers
            .meshes
            .import(&obj, b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n")
            .expect("a triangle");
        assert_eq!(mesh.indices.len(), 3);
        assert_eq!(importers.images.names().len(), 4);
    }

    #[test]
    fn a_name_of_another_kind_is_unknown() {
        let importers = Importers::standard().expect("unique names");
        let gltf = AssetMeta::new(AssetId::from_bits(1), name("gltf").unwrap());
        assert!(importers.meshes.import(&gltf, b"").is_err());
    }
}

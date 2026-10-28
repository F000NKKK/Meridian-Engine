//! The windowed application's graphics session: the wgpu recorder, the
//! registries and the asset cache an application builds once. The mesh
//! builders it used to hold are `meridian_rendering_domain`'s `shapes`; the
//! per-frame logic is `meridian_rendering_app::RenderSession`.

use meridian_asset_domain::{Decoder, HdrDecoder};
use meridian_rendering_app::{FrameRecorder, RenderSession};
use meridian_rendering_domain::{CubeFaces, EnvironmentMap, MeshHandle, TextureHandle};
use meridian_rendering_wgpu::{Device, GpuFrameRecorder, Surface};

use crate::any_image_decoder::AnyImageDecoder;
use crate::assets::AssetCache;
use crate::model_loading::LoadedModel;

/// What a windowed application builds once: a [`RenderSession`] over the wgpu
/// [`GpuFrameRecorder`] (device, swapchain, renderer, bloom, textures, plus the
/// mesh and material registries to fill) and the asset cache.
///
/// Constructed from an already-open `Device`/`Surface` (the windowed handshake
/// itself stays in the application's own `on_ready`, the only place that needs
/// to name `winit`/`wgpu`-adjacent types at all). Loading goes through the
/// forwarding methods [`load_texture`](Self::load_texture) and
/// [`load_mesh_obj`](Self::load_mesh_obj), whose decode/cache/register logic
/// lives in [`crate::assets`].
#[derive(Debug)]
pub struct GraphicsSession {
    pub render: RenderSession<GpuFrameRecorder>,
    pub assets: AssetCache,
}

impl GraphicsSession {
    /// Builds with the default shadow map resolution — see
    /// [`with_shadow_map_size`](Self::with_shadow_map_size) to pick a specific
    /// one (sharper-but-costlier vs. cheaper-but-blockier shadows).
    #[must_use]
    pub fn new(device: Device, surface: Surface, width: u32, height: u32) -> Self {
        Self::with_shadow_map_size(
            device,
            surface,
            width,
            height,
            meridian_rendering_wgpu::DEFAULT_SHADOW_MAP_SIZE,
        )
    }

    /// Builds with a chosen shadow map resolution.
    #[must_use]
    pub fn with_shadow_map_size(
        device: Device,
        surface: Surface,
        width: u32,
        height: u32,
        shadow_map_size: u32,
    ) -> Self {
        let recorder =
            GpuFrameRecorder::with_shadow_map_size(device, surface, width, height, shadow_map_size);
        Self {
            render: RenderSession::new(recorder),
            assets: AssetCache::new(),
        }
    }

    /// Rebuilds every size-dependent piece after a window resize.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.render.resize(width, height);
    }

    /// Forwards to [`AssetCache::load_texture`].
    pub fn load_texture(&mut self, path: &str) -> TextureHandle {
        self.assets.load_texture(&mut self.render.recorder, path)
    }

    /// Forwards to [`AssetCache::load_model_gltf`].
    ///
    /// # Errors
    ///
    /// The file's read, decode or registration error, as text.
    pub fn load_model_gltf(&mut self, path: &str) -> Result<LoadedModel, String> {
        self.assets
            .load_model_gltf(&mut self.render.meshes, &mut self.render.recorder, path)
    }

    /// Forwards to [`AssetCache::load_mesh_obj`].
    ///
    /// # Errors
    ///
    /// The file's read, decode or mesh-validation error, as text.
    pub fn load_mesh_obj(&mut self, path: &str) -> Result<MeshHandle, String> {
        self.assets.load_mesh_obj(&mut self.render.meshes, path)
    }

    /// Lights the scene with the equirectangular Radiance `.hdr` panorama at
    /// `path` (ADR 036): decoded, prefiltered and handed to the recorder.
    ///
    /// # Errors
    ///
    /// The file's read, decode or validation error, as text.
    pub fn load_environment(&mut self, path: &str) -> Result<(), String> {
        let map = load_environment_map(path)?;
        self.render.recorder.set_environment(Some(&map));
        Ok(())
    }

    /// Lights the scene with the cube map whose six faces are the image files
    /// `paths`, in the order `+x, -x, +y, -y, +z, -z` (ADR 038). Faces are
    /// Radiance `.hdr` files or ordinary images, which are read as sRGB.
    ///
    /// # Errors
    ///
    /// A face's read or decode error, faces that are not square or not all the
    /// same size, or a validation error, as text.
    pub fn load_environment_cube(&mut self, paths: [&str; 6]) -> Result<(), String> {
        let map = load_environment_cube(paths)?;
        self.render.recorder.set_environment(Some(&map));
        Ok(())
    }
}

/// Reads the equirectangular `.hdr` panorama at `path` and prefilters it.
///
/// # Errors
///
/// The file's read, decode or environment-validation error, as text.
fn load_environment_map(path: &str) -> Result<EnvironmentMap, String> {
    let image = read_radiance(path)?;
    EnvironmentMap::from_equirect(image.width, image.height, &image.texels)
        .map_err(|e| format!("invalid environment {path}: {e}"))
}

/// Reads the six face files of a cube map, in the order `+x, -x, +y, -y, +z,
/// -z`, and prefilters it.
///
/// # Errors
///
/// A face's read or decode error, faces that are not square or not all the
/// same size, or a validation error, as text.
fn load_environment_cube(paths: [&str; 6]) -> Result<EnvironmentMap, String> {
    let faces = paths.map(read_radiance);
    let mut images = Vec::with_capacity(6);
    for face in faces {
        images.push(face?);
    }
    let size = images[0].width;
    for (image, path) in images.iter().zip(paths) {
        if image.width != size || image.height != size {
            return Err(format!(
                "cube face {path} is {}x{}, expected {size}x{size}",
                image.width, image.height
            ));
        }
    }
    let texels: Vec<Vec<[f32; 3]>> = images.into_iter().map(|image| image.texels).collect();
    let six = <[Vec<[f32; 3]>; 6]>::try_from(texels)
        .map_err(|_| "cube map needs six faces".to_string())?;
    let cube = CubeFaces::new(size, six).map_err(|err| format!("invalid cube map: {err}"))?;
    EnvironmentMap::from_cube(&cube).map_err(|err| format!("invalid cube map: {err}"))
}

/// A decoded image of linear radiance.
struct RadianceImage {
    width: u32,
    height: u32,
    texels: Vec<[f32; 3]>,
}

/// Reads `path` as a Radiance file when it starts with the signature, else as
/// an 8-bit image in sRGB.
fn read_radiance(path: &str) -> Result<RadianceImage, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("failed to read {path}: {e}"))?;
    if HdrDecoder::detect(&bytes) {
        let image = HdrDecoder
            .decode(&bytes)
            .map_err(|e| format!("failed to decode {path}: {e}"))?;
        return Ok(RadianceImage {
            width: image.width,
            height: image.height,
            texels: image.pixels,
        });
    }
    let image = AnyImageDecoder
        .decode(&bytes)
        .map_err(|e| format!("failed to decode {path}: {e}"))?;
    let texels = image
        .pixels
        .chunks_exact(4)
        .map(|rgba| [rgba[0], rgba[1], rgba[2]].map(srgb_to_linear))
        .collect();
    Ok(RadianceImage {
        width: image.width,
        height: image.height,
        texels,
    })
}

/// The linear value of an sRGB-encoded byte.
fn srgb_to_linear(byte: u8) -> f32 {
    let encoded = f32::from(byte) / 255.0;
    if encoded <= 0.04045 {
        encoded / 12.92
    } else {
        ((encoded + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_file(name: &str, bytes: &[u8]) -> String {
        let path = std::env::temp_dir().join(format!("meridian-sdk-{}-{name}", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        path.to_string_lossy().into_owned()
    }

    fn remove_all(paths: Vec<String>) {
        for path in paths {
            std::fs::remove_file(path).unwrap();
        }
    }

    /// A flat-coded Radiance file of `width` by `height` mid-grey texels.
    fn hdr_file(width: usize, height: usize) -> Vec<u8> {
        let mut file =
            format!("#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y {height} +X {width}\n").into_bytes();
        file.extend(std::iter::repeat_n([128u8, 128, 128, 129], width * height).flatten());
        file
    }

    #[test]
    fn a_panorama_file_becomes_an_environment() {
        let path = write_file("ok.hdr", &hdr_file(8, 4));
        assert!(load_environment_map(&path).is_ok());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn missing_and_malformed_files_are_errors() {
        assert!(load_environment_map("/nonexistent/sky.hdr").is_err());
        let path = write_file("bad.hdr", b"not a radiance file");
        assert!(load_environment_map(&path).unwrap_err().contains("decode"));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn six_square_faces_become_an_environment() {
        let paths: Vec<String> = (0..6)
            .map(|i| write_file(&format!("face{i}.hdr"), &hdr_file(4, 4)))
            .collect();
        let refs: [&str; 6] = std::array::from_fn(|i| paths[i].as_str());
        assert!(load_environment_cube(refs).is_ok());
        remove_all(paths);
    }

    #[test]
    fn faces_of_different_sizes_are_refused() {
        let paths: Vec<String> = (0..6)
            .map(|i| {
                write_file(
                    &format!("mixed{i}.hdr"),
                    &hdr_file(if i == 3 { 8 } else { 4 }, if i == 3 { 8 } else { 4 }),
                )
            })
            .collect();
        let refs: [&str; 6] = std::array::from_fn(|i| paths[i].as_str());
        let err = load_environment_cube(refs).unwrap_err();
        assert!(err.contains("expected 4x4"), "{err}");
        remove_all(paths);
    }

    #[test]
    fn non_square_faces_are_refused() {
        let paths: Vec<String> = (0..6)
            .map(|i| write_file(&format!("wide{i}.hdr"), &hdr_file(8, 4)))
            .collect();
        let refs: [&str; 6] = std::array::from_fn(|i| paths[i].as_str());
        assert!(load_environment_cube(refs).is_err());
        remove_all(paths);
    }

    #[test]
    fn srgb_bytes_decode_to_linear_values() {
        assert!((srgb_to_linear(0)).abs() < 1e-7);
        assert!((srgb_to_linear(255) - 1.0).abs() < 1e-6);
        assert!((srgb_to_linear(128) - 0.2158).abs() < 1e-3);
    }
}

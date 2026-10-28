//! Radiance `.hdr` file to [`EnvironmentMap`] (ADR 036): the read and the
//! decode live here, the prefiltering in `rendering-domain`.

use meridian_asset_domain::{Decoder, HdrDecoder};
use meridian_rendering_domain::EnvironmentMap;

/// Reads the equirectangular `.hdr` panorama at `path` and prefilters it.
///
/// # Errors
///
/// The file's read, decode or environment-validation error, as text.
pub(crate) fn load_environment_map(path: &str) -> Result<EnvironmentMap, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("failed to read {path}: {e}"))?;
    let image = HdrDecoder
        .decode(&bytes)
        .map_err(|e| format!("failed to decode {path}: {e}"))?;
    EnvironmentMap::from_equirect(image.width, image.height, &image.pixels)
        .map_err(|e| format!("invalid environment {path}: {e:?}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_hdr(name: &str, bytes: &[u8]) -> String {
        let path = std::env::temp_dir().join(format!("meridian-sdk-{}-{name}", std::process::id()));
        std::fs::write(&path, bytes).unwrap();
        path.to_string_lossy().into_owned()
    }

    #[test]
    fn a_panorama_file_becomes_an_environment() {
        let mut file = b"#?RADIANCE\nFORMAT=32-bit_rle_rgbe\n\n-Y 4 +X 8\n".to_vec();
        file.extend(std::iter::repeat_n([128u8, 128, 128, 129], 32).flatten());
        let path = write_hdr("ok.hdr", &file);
        assert!(load_environment_map(&path).is_ok());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn missing_and_malformed_files_are_errors() {
        assert!(load_environment_map("/nonexistent/sky.hdr").is_err());
        let path = write_hdr("bad.hdr", b"not a radiance file");
        assert!(load_environment_map(&path).unwrap_err().contains("decode"));
        std::fs::remove_file(path).unwrap();
    }
}

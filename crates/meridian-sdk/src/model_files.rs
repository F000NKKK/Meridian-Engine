//! The external files a `.gltf` names, read from beside it.

use std::path::{Component, Path, PathBuf};

use meridian_asset_gltf::Resources;

/// Reads every file in `uris` from `base`, the directory of the model
/// file, keyed by the URI the model uses.
///
/// # Errors
///
/// A URI that escapes `base` (a scheme, an absolute path, `..`), or a file
/// that cannot be read.
pub(crate) fn read_all(base: &Path, uris: &[String]) -> Result<Resources, String> {
    let mut resources = Resources::new();
    for uri in uris {
        let path = local_path(base, uri)?;
        let bytes = std::fs::read(&path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        resources.insert(uri.clone(), bytes);
    }
    Ok(resources)
}

/// The file `uri` names under `base`. glTF URIs are relative and
/// percent-encoded; anything that could leave `base` is refused.
fn local_path(base: &Path, uri: &str) -> Result<PathBuf, String> {
    let decoded = percent_decode(uri).ok_or_else(|| format!("bad escape in URI `{uri}`"))?;
    let relative = Path::new(&decoded);
    let stays_inside = relative
        .components()
        .all(|part| matches!(part, Component::Normal(_) | Component::CurDir));
    if decoded.contains(':') || !stays_inside {
        return Err(format!("URI `{uri}` does not name a file beside the model"));
    }
    Ok(base.join(relative))
}

/// `%20`-style escapes undone; `None` for a malformed escape or bytes that
/// are not UTF-8.
fn percent_decode(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            let pair = text.get(i + 1..i + 3)?;
            out.push(u8::from_str_radix(pair, 16).ok()?);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_relative_uri_resolves_under_the_base() {
        let path = local_path(Path::new("/models"), "textures/wall%20a.png").unwrap();
        assert_eq!(path, Path::new("/models/textures/wall a.png"));
    }

    #[test]
    fn uris_that_leave_the_base_are_refused() {
        for uri in [
            "../secret.bin",
            "/etc/passwd",
            "http://host/a.bin",
            "a/../../b",
            "%2e%2e/x",
        ] {
            assert!(local_path(Path::new("/models"), uri).is_err(), "{uri}");
        }
    }

    #[test]
    fn a_malformed_escape_is_refused() {
        assert!(local_path(Path::new("/m"), "a%zz").is_err());
        assert!(local_path(Path::new("/m"), "a%2").is_err());
    }

    #[test]
    fn read_all_loads_each_named_file() {
        let dir = std::env::temp_dir().join(format!("meridian-model-files-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("tex")).unwrap();
        std::fs::write(dir.join("scene.bin"), [1, 2]).unwrap();
        std::fs::write(dir.join("tex/a b.png"), [3]).unwrap();
        let uris = ["scene.bin".to_owned(), "tex/a%20b.png".to_owned()];
        let resources = read_all(&dir, &uris).unwrap();
        assert_eq!(resources["scene.bin"], [1, 2]);
        assert_eq!(resources["tex/a%20b.png"], [3]);
        assert!(read_all(&dir, &["missing.bin".to_owned()]).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

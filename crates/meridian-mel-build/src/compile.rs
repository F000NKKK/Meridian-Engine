//! Compiling `.mel` files into `.g.rs` files from a build script.

use std::path::{Path, PathBuf};

use crate::{BuildError, GeneratedModule, MelDocument};

/// Compiles every `*.mel` under `dir` (recursively) into
/// `$OUT_DIR/mel/<path relative to dir>.g.rs`, so
/// `assets/scenes/hud/main.mel` becomes `mel_include!("hud/main")`, and
/// asks Cargo to rerun the build script when anything under `dir`
/// changes. Returns the generated files.
pub fn compile_dir(dir: impl AsRef<Path>) -> Result<Vec<PathBuf>, BuildError> {
    let dir = dir.as_ref();
    let out_dir = std::env::var_os("OUT_DIR").ok_or(BuildError::OutDirMissing)?;
    let out_dir = PathBuf::from(out_dir).join("mel");
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut sources = Vec::new();
    find_mel_files(dir, &mut sources)?;
    sources.sort();
    sources
        .iter()
        .map(|source| {
            let relative = source.strip_prefix(dir).unwrap_or(source);
            compile_file(source, &out_dir.join(relative).with_extension("g.rs"))
        })
        .collect()
}

/// Compiles one `.mel` file into `target`, creating its directory.
pub fn compile_file(source: &Path, target: &Path) -> Result<PathBuf, BuildError> {
    let source = source.canonicalize().map_err(|error| io(source, error))?;
    let text = std::fs::read_to_string(&source).map_err(|error| io(&source, error))?;
    let document = MelDocument::parse(&source, &text)?;
    let module = GeneratedModule::render(&document);
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|error| io(parent, error))?;
    }
    std::fs::write(target, module.code).map_err(|error| io(target, error))?;
    Ok(target.to_path_buf())
}

fn find_mel_files(dir: &Path, found: &mut Vec<PathBuf>) -> Result<(), BuildError> {
    let entries = std::fs::read_dir(dir).map_err(|error| io(dir, error))?;
    for entry in entries {
        let path = entry.map_err(|error| io(dir, error))?.path();
        if path.is_dir() {
            find_mel_files(&path, found)?;
        } else if path.extension().is_some_and(|ext| ext == "mel") {
            found.push(path);
        }
    }
    Ok(())
}

fn io(path: &Path, error: std::io::Error) -> BuildError {
    BuildError::Io {
        path: path.to_path_buf(),
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("meridian-mel-build-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn compile_file_writes_the_module() {
        let dir = scratch("file");
        let source = dir.join("hud.mel");
        std::fs::write(&source, r#"<Scene><Entity x:Name="sun" /></Scene>"#).unwrap();
        let target = dir.join("out/hud.g.rs");
        compile_file(&source, &target).unwrap();
        let code = std::fs::read_to_string(&target).unwrap();
        assert!(code.contains("Sun,"), "{code}");
    }

    #[test]
    fn malformed_file_fails_with_its_path() {
        let dir = scratch("bad");
        let source = dir.join("bad.mel");
        std::fs::write(&source, "<Scene>").unwrap();
        let error = compile_file(&source, &dir.join("bad.g.rs")).unwrap_err();
        assert!(error.to_string().contains("bad.mel:1:"), "{error}");
        assert!(!dir.join("bad.g.rs").exists());
    }

    #[test]
    fn missing_file_is_an_io_error() {
        let dir = scratch("missing");
        let error = compile_file(&dir.join("nope.mel"), &dir.join("nope.g.rs")).unwrap_err();
        assert!(matches!(error, BuildError::Io { .. }));
    }
}

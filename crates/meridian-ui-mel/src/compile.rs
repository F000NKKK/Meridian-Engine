//! Compiling `.meui` and `.mres` files into `.g.rs` files from a build
//! script.

use std::path::{Path, PathBuf};

use crate::source_dir::{SourceFiles, insert_set, read_set};
use crate::{BuildError, GeneratedModule, GeneratedResources, MelDocument, ResourceSets};

/// Compiles every `*.mres` and `*.meui` under `dir` (recursively) into
/// `$OUT_DIR/mel/<path relative to dir>.g.rs`, so
/// `assets/scenes/hud/main.meui` becomes `mel_include!("hud/main")` and
/// `assets/scenes/Strings.mres` becomes `mel_include!("Strings")`, and
/// asks Cargo to rerun the build script when anything under `dir`
/// changes. Resource sets are read first: a `.meui` may name any of them
/// with `@res:Set.name`. Returns the generated files.
pub fn compile_dir(dir: impl AsRef<Path>) -> Result<Vec<PathBuf>, BuildError> {
    let dir = dir.as_ref();
    let out_dir = std::env::var_os("OUT_DIR").ok_or(BuildError::OutDirMissing)?;
    println!("cargo:rerun-if-changed={}", dir.display());
    compile_tree(dir, &PathBuf::from(out_dir).join("mel"))
}

/// [`compile_dir`] with the output directory given.
fn compile_tree(dir: &Path, out_dir: &Path) -> Result<Vec<PathBuf>, BuildError> {
    let files = SourceFiles::find(dir)?;
    let target = |source: &Path| -> Result<PathBuf, BuildError> {
        let relative = source
            .strip_prefix(dir)
            .map_err(|_| BuildError::OutsideDir {
                file: source.to_path_buf(),
                dir: dir.to_path_buf(),
            })?;
        Ok(out_dir.join(relative).with_extension("g.rs"))
    };
    let mut sets = ResourceSets::default();
    let mut generated = Vec::new();
    for source in &files.mres {
        let set = read_set(source)?;
        if set.culture.is_none() {
            let file = format!("{}.mres", set.name);
            write(
                &target(source)?,
                &GeneratedResources::render(&set, &file).code,
                &mut generated,
            )?;
        }
        insert_set(&mut sets, set, source)?;
    }
    for source in &files.meui {
        generated.push(compile_file_with(source, &target(source)?, &sets)?);
    }
    Ok(generated)
}

/// Compiles one `.meui` file into `target`, creating its directory.
pub fn compile_file(source: &Path, target: &Path) -> Result<PathBuf, BuildError> {
    compile_file_with(source, target, &ResourceSets::default())
}

/// Like [`compile_file`], resolving `@res:Set.name` against `resources`.
pub fn compile_file_with(
    source: &Path,
    target: &Path,
    resources: &ResourceSets,
) -> Result<PathBuf, BuildError> {
    let source = source
        .canonicalize()
        .map_err(|error| BuildError::io(source, error))?;
    let text = std::fs::read_to_string(&source).map_err(|error| BuildError::io(&source, error))?;
    let document = MelDocument::parse_with(&source, &text, resources)?;
    let module = GeneratedModule::render(&document);
    write(target, &module.code, &mut Vec::new())?;
    Ok(target.to_path_buf())
}

fn write(target: &Path, code: &str, written: &mut Vec<PathBuf>) -> Result<(), BuildError> {
    if let Some(parent) = target.parent() {
        std::fs::create_dir_all(parent).map_err(|error| BuildError::io(parent, error))?;
    }
    std::fs::write(target, code).map_err(|error| BuildError::io(target, error))?;
    written.push(target.to_path_buf());
    Ok(())
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
        let source = dir.join("hud.meui");
        std::fs::write(&source, r#"<Scene><Entity x:Name="sun" /></Scene>"#).unwrap();
        let target = dir.join("out/hud.g.rs");
        compile_file(&source, &target).unwrap();
        let code = std::fs::read_to_string(&target).unwrap();
        assert!(code.contains("Sun,"), "{code}");
    }

    #[test]
    fn malformed_file_fails_with_its_path() {
        let dir = scratch("bad");
        let source = dir.join("bad.meui");
        std::fs::write(&source, "<Scene>").unwrap();
        let error = compile_file(&source, &dir.join("bad.g.rs")).unwrap_err();
        assert!(error.to_string().contains("bad.meui:1:"), "{error}");
        assert!(!dir.join("bad.g.rs").exists());
    }

    #[test]
    fn compile_dir_outside_a_build_script_is_an_error() {
        // Cargo sets OUT_DIR only while compiling, never for a test run.
        let error = compile_dir(scratch("no-out-dir")).unwrap_err();
        assert!(matches!(error, BuildError::OutDirMissing));
    }

    #[test]
    fn directory_compiles_resources_and_markup_together() {
        let dir = scratch("tree");
        std::fs::create_dir_all(dir.join("ui")).unwrap();
        std::fs::write(dir.join("ui/Strings.mres"), r#"{ title: "Yog" }"#).unwrap();
        std::fs::write(
            dir.join("ui/hud.meui"),
            r#"<TextBlock x:Name="label" text="@res:Strings.title" />"#,
        )
        .unwrap();
        let out = dir.join("out");
        let files = compile_tree(&dir, &out).unwrap();
        assert_eq!(files.len(), 2);
        let strings = std::fs::read_to_string(out.join("ui/Strings.g.rs")).unwrap();
        assert!(
            strings.contains("pub const TITLE: &'static str = \"Yog\";"),
            "{strings}"
        );
        let hud = std::fs::read_to_string(out.join("ui/hud.g.rs")).unwrap();
        assert!(hud.contains(r#""Yog")?;"#), "{hud}");
    }

    #[test]
    fn variants_generate_nothing_and_are_checked() {
        let dir = scratch("variants");
        std::fs::write(dir.join("Strings.mres"), r#"{ title: "Yog" }"#).unwrap();
        std::fs::write(dir.join("Strings.ru.mres"), r#"{ title: "Йог" }"#).unwrap();
        let files = compile_tree(&dir, &dir.join("out")).unwrap();
        assert_eq!(files.len(), 1, "{files:?}");
        std::fs::write(dir.join("Strings.ru.mres"), r#"{ nope: "x" }"#).unwrap();
        let error = compile_tree(&dir, &dir.join("out")).unwrap_err();
        assert!(
            error.to_string().contains("which the default set does not"),
            "{error}"
        );
        std::fs::write(dir.join("Strings.ru.mres"), "{}").unwrap();
        std::fs::write(dir.join("Strings.r_u.mres"), "{}").unwrap();
        let error = compile_tree(&dir, &dir.join("out")).unwrap_err();
        assert!(error.to_string().contains("is not a culture"), "{error}");
    }

    #[test]
    fn unknown_resource_and_duplicate_sets_fail_the_build() {
        let dir = scratch("bad-res");
        std::fs::write(dir.join("a.meui"), r#"<TextBlock text="@res:Nope.x" />"#).unwrap();
        let error = compile_tree(&dir, &dir.join("out")).unwrap_err();
        assert!(
            error.to_string().contains("no resource 'Nope.x'"),
            "{error}"
        );

        let dir = scratch("dup-set");
        std::fs::create_dir_all(dir.join("a")).unwrap();
        std::fs::create_dir_all(dir.join("b")).unwrap();
        std::fs::write(dir.join("a/Strings.mres"), "{}").unwrap();
        std::fs::write(dir.join("b/Strings.mres"), "{}").unwrap();
        let error = compile_tree(&dir, &dir.join("out")).unwrap_err();
        assert!(error.to_string().contains("defined twice"), "{error}");
    }

    #[test]
    fn missing_file_is_an_io_error() {
        let dir = scratch("missing");
        let error = compile_file(&dir.join("nope.meui"), &dir.join("nope.g.rs")).unwrap_err();
        assert!(matches!(error, BuildError::Io { .. }));
    }
}

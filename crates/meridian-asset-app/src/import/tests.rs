//! Registry and loader against in-memory fakes.

use std::sync::Arc;

use meridian_asset_domain::{AssetId, AssetMeta, Decoder, ImporterName};

use super::{ImportError, ImportFailure, Importer, ImporterRegistry, RegisterError};
use crate::memory_source::MemorySource;
use crate::{AssetLoader, AssetPath, LoadError, Priority, SourceError, Spawn};

struct Inline;

impl Spawn for Inline {
    fn spawn(&self, _priority: Priority, job: Box<dyn FnOnce() + Send>) {
        job();
    }
}

#[derive(Debug, PartialEq)]
struct Text(String);

#[derive(Debug, thiserror::Error)]
#[error("not utf-8")]
struct NotUtf8;

struct Utf8;

impl Decoder<Text> for Utf8 {
    type Error = NotUtf8;
    fn decode(&self, bytes: &[u8]) -> Result<Text, NotUtf8> {
        String::from_utf8(bytes.to_vec())
            .map(Text)
            .map_err(|_| NotUtf8)
    }
}

/// Repeats the text `times` as the `.meta` settings say.
struct Repeating;

impl Importer<Text> for Repeating {
    fn import(&self, bytes: &[u8], meta: &AssetMeta) -> Result<Text, ImportFailure> {
        let times = match meta.setting("times") {
            Some(value) => value.as_i64().ok_or("times is not an integer")?,
            None => 1,
        };
        let text = String::from_utf8(bytes.to_vec())?;
        Ok(Text(text.repeat(usize::try_from(times)?)))
    }
}

fn name(text: &str) -> ImporterName {
    ImporterName::new(text).unwrap()
}

fn meta_for(importer: &str) -> AssetMeta {
    AssetMeta::new(AssetId::from_bits(1), name(importer))
}

fn registry() -> ImporterRegistry<Text> {
    let mut registry = ImporterRegistry::new();
    registry.register_decoder(name("utf8"), Utf8).unwrap();
    registry.register(name("repeat"), Repeating).unwrap();
    registry
}

#[test]
fn a_name_resolves_to_its_importer() {
    let registry = registry();
    let text = registry.import(&meta_for("utf8"), b"hi").unwrap();
    assert_eq!(text, Text("hi".into()));
    assert_eq!(registry.names(), vec![&name("repeat"), &name("utf8")]);
}

#[test]
fn an_unknown_name_is_an_error_not_a_panic() {
    let error = registry().import(&meta_for("fbx"), b"x").unwrap_err();
    assert!(matches!(error, ImportError::UnknownImporter(n) if n == name("fbx")));
}

#[test]
fn a_name_cannot_be_registered_twice() {
    let mut registry = registry();
    let error = registry.register_decoder(name("utf8"), Utf8).unwrap_err();
    assert_eq!(error, RegisterError::NameTaken(name("utf8")));
}

#[test]
fn an_importer_failure_names_the_importer() {
    let error = registry()
        .import(&meta_for("utf8"), &[0xff, 0xfe])
        .unwrap_err();
    assert!(matches!(&error, ImportError::Failed { importer, .. } if *importer == name("utf8")));
    assert!(error.to_string().contains("not utf-8"));
}

#[test]
fn an_importer_reads_its_settings() {
    let meta = AssetMeta::read(
        br#"{ format: 1, id: "0000000000000001", importer: "repeat", settings: { times: 3 } }"#,
    )
    .unwrap();
    assert_eq!(
        registry().import(&meta, b"ab").unwrap(),
        Text("ababab".into())
    );
    let bad = AssetMeta::read(
        br#"{ format: 1, id: "0000000000000001", importer: "repeat", settings: { times: -1 } }"#,
    )
    .unwrap();
    assert!(matches!(
        registry().import(&bad, b"ab"),
        Err(ImportError::Failed { .. })
    ));
}

fn loader(source: MemorySource) -> AssetLoader {
    AssetLoader::new(Arc::new(source), Arc::new(Inline))
}

const META: &[u8] = br#"{ format: 1, id: "0000000000000001", importer: "utf8" }"#;

#[test]
fn the_loader_reads_the_meta_then_imports() {
    let loader = loader(
        MemorySource::default()
            .with("a.txt", b"hello")
            .with("a.txt.meta", META),
    );
    let text = loader
        .load_imported(AssetPath::new("a.txt"), Arc::new(registry()))
        .wait()
        .unwrap();
    assert_eq!(text, Text("hello".into()));
}

#[test]
fn a_missing_sidecar_is_not_found() {
    let loader = loader(MemorySource::default().with("a.txt", b"hello"));
    let result = loader
        .load_imported(AssetPath::new("a.txt"), Arc::new(registry()))
        .wait();
    assert!(matches!(
        result,
        Err(LoadError::Source(SourceError::NotFound(p))) if p.as_str() == "a.txt.meta"
    ));
}

#[test]
fn a_damaged_sidecar_or_unknown_importer_is_a_decode_error() {
    let loader = loader(
        MemorySource::default()
            .with("a.txt", b"x")
            .with("a.txt.meta", b"{ not a meta")
            .with("b.txt", b"x")
            .with(
                "b.txt.meta",
                br#"{ format: 1, id: "0000000000000002", importer: "nope" }"#,
            ),
    );
    let registry = Arc::new(registry());
    let damaged = loader
        .load_imported(AssetPath::new("a.txt"), Arc::clone(&registry))
        .wait();
    assert!(matches!(
        damaged,
        Err(LoadError::Decode(ImportError::InvalidMeta { .. }))
    ));
    let unknown = loader
        .load_imported(AssetPath::new("b.txt"), registry)
        .wait();
    assert!(matches!(
        unknown,
        Err(LoadError::Decode(ImportError::UnknownImporter(_)))
    ));
}

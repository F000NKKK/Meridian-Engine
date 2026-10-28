//! Stacked sources: priority, fall-through and damage.

use std::sync::Arc;

use crate::source::MemorySource;
use crate::*;

/// A source whose every read fails the way a damaged pack does.
struct Damaged;

impl ByteSource for Damaged {
    fn read(&self, path: &AssetPath) -> Result<Vec<u8>, SourceError> {
        Err(SourceError::Unreadable {
            path: path.clone(),
            reason: "checksum".into(),
        })
    }
}

impl AssetListing for Damaged {
    fn paths(&self) -> Result<Vec<AssetPath>, SourceError> {
        Ok(vec![AssetPath::new("a.bin")])
    }
}

fn stack(layers: Vec<Arc<dyn AssetListing>>) -> OverlaySource {
    OverlaySource::new(layers)
}

fn base_and_patch() -> OverlaySource {
    stack(vec![
        Arc::new(
            MemorySource::default()
                .with("a.bin", b"base-a")
                .with("b.bin", b"base-b"),
        ),
        Arc::new(
            MemorySource::default()
                .with("b.bin", b"patch-b")
                .with("c.bin", b"patch-c"),
        ),
    ])
}

#[test]
fn the_top_layer_wins_and_the_rest_fall_through() {
    let source = base_and_patch();
    let read = |text: &str| source.read(&AssetPath::new(text));
    assert_eq!(read("a.bin"), Ok(b"base-a".to_vec()));
    assert_eq!(read("b.bin"), Ok(b"patch-b".to_vec()));
    assert_eq!(read("c.bin"), Ok(b"patch-c".to_vec()));
}

#[test]
fn a_path_in_no_layer_is_not_found() {
    let source = base_and_patch();
    let path = AssetPath::new("nope.bin");
    assert_eq!(source.read(&path), Err(SourceError::NotFound(path)));
    let empty = stack(Vec::new());
    assert!(empty.is_empty());
    assert!(matches!(
        empty.read(&AssetPath::new("a")),
        Err(SourceError::NotFound(_))
    ));
}

#[test]
fn the_listing_is_the_sorted_union() {
    let paths = base_and_patch().paths().unwrap();
    let texts: Vec<_> = paths.iter().map(AssetPath::as_str).collect();
    assert_eq!(texts, ["a.bin", "b.bin", "c.bin"]);
}

#[test]
fn damage_in_the_top_layer_is_not_hidden_by_the_one_below() {
    let source = stack(vec![
        Arc::new(MemorySource::default().with("a.bin", b"old")),
        Arc::new(Damaged),
    ]);
    assert!(matches!(
        source.read(&AssetPath::new("a.bin")),
        Err(SourceError::Unreadable { .. })
    ));
}

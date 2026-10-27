//! Tests for [`LiveUi::reload`].

use meridian_dsl_core::parse;
use meridian_ui_core::layout::value::Length;
use meridian_ui_core::{NodeId, UiTree, properties};

use super::*;
use crate::{ResourceSets, UiNode};

fn node(source: &str) -> UiNode {
    let root = parse(source).unwrap();
    UiNode::from_element(&root, &[], &ResourceSets::default()).unwrap()
}

/// What the generated `build_ui` does.
fn build(tree: &mut UiTree, ui: &UiNode) -> NodeId {
    super::tree_builder::build(tree, ui, &mut Vec::new()).unwrap()
}

fn live(source: &str) -> (UiTree, NodeId, LiveUi) {
    let ui = node(source);
    let mut tree = UiTree::new();
    let root = build(&mut tree, &ui);
    (tree, root, LiveUi::new(root, ui))
}

const BEFORE: &str = r#"<StackPanel width="200">
    <TextBlock text="Hi" font_size="12" />
</StackPanel>"#;

#[test]
fn changed_attributes_are_set_in_place() {
    let (mut tree, root, mut live) = live(BEFORE);
    let text = tree.children(root).unwrap()[0];
    let after = BEFORE.replace("Hi", "Hello").replace("200", "300");
    let outcome = live.reload(&mut tree, node(&after)).unwrap();
    assert_eq!(outcome, Reload::Patched { changed: 2 });
    assert_eq!(
        tree.get(root, properties::element::WIDTH).unwrap(),
        Length::Px(300.0)
    );
    assert_eq!(
        tree.get(text, properties::text_block::TEXT).unwrap(),
        "Hello"
    );
    assert_eq!(tree.children(root).unwrap()[0], text);
}

#[test]
fn an_unchanged_document_changes_nothing() {
    let (mut tree, _, mut live) = live(BEFORE);
    tree.drain_changes();
    let outcome = live.reload(&mut tree, node(BEFORE)).unwrap();
    assert_eq!(outcome, Reload::Patched { changed: 0 });
    assert!(tree.drain_changes().is_empty());
}

#[test]
fn a_removed_attribute_goes_back_to_the_default() {
    let (mut tree, root, mut live) = live(BEFORE);
    let after = BEFORE.replace(r#" width="200""#, "");
    let outcome = live.reload(&mut tree, node(&after)).unwrap();
    assert_eq!(outcome, Reload::Patched { changed: 1 });
    let default = (properties::element::WIDTH.meta().default)();
    assert_eq!(
        tree.value_of(root, properties::element::WIDTH.meta().id)
            .unwrap(),
        default
    );
}

#[test]
fn a_new_shape_leaves_the_tree_alone() {
    let (mut tree, root, mut live) = live(BEFORE);
    tree.drain_changes();
    let after = BEFORE
        .replace("<TextBlock", "<StackPanel")
        .replace(r#" text="Hi" font_size="12" />"#, " />");
    assert_eq!(
        live.reload(&mut tree, node(&after)).unwrap(),
        Reload::Restructured
    );
    let extra = r#"<StackPanel width="200"><TextBlock text="Hi" font_size="12" /><TextBlock text="2" /></StackPanel>"#;
    assert_eq!(
        live.reload(&mut tree, node(extra)).unwrap(),
        Reload::Restructured
    );
    assert!(tree.drain_changes().is_empty());
    assert_eq!(
        tree.get(root, properties::element::WIDTH).unwrap(),
        Length::Px(200.0)
    );
    // Still the old document: a matching reload patches.
    assert_eq!(
        live.reload(&mut tree, node(BEFORE)).unwrap(),
        Reload::Patched { changed: 0 }
    );
}

mod files {
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    use super::*;
    use crate::{HotDocument, Refreshed, ReloadError, SourceWatcher};

    const DEBOUNCE: Duration = Duration::from_millis(50);

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("mel-hot-{name}-{}", std::process::id()));
        if dir.exists() {
            std::fs::remove_dir_all(&dir).unwrap();
        }
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write(path: &Path, text: &str) {
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn a_change_is_reported_once_after_it_settles() {
        let dir = scratch("watch");
        let file = dir.join("a.meui");
        write(&file, "one");
        let mut watcher = SourceWatcher::new([file.clone()], DEBOUNCE);
        let start = Instant::now();
        assert!(watcher.poll(start).is_empty());
        write(&file, "two, longer");
        assert!(watcher.poll(start).is_empty(), "inside the debounce");
        write(&file, "three, longer still");
        let later = start + DEBOUNCE / 2;
        assert!(watcher.poll(later).is_empty(), "a new write restarts it");
        let settled = later + DEBOUNCE * 2;
        assert_eq!(watcher.poll(settled), [file]);
        assert!(watcher.poll(settled + DEBOUNCE).is_empty());
    }

    fn open(dir: &Path) -> (UiTree, NodeId, HotDocument) {
        write(&dir.join("Strings.mres"), r#"{ title: "Yog" }"#);
        let meui = dir.join("hud.meui");
        write(
            &meui,
            r#"<StackPanel width="200"><TextBlock x:Name="label" text="@res:Strings.title" /></StackPanel>"#,
        );
        // The tree is built from the same files, as `build_ui` would.
        let mut tree = UiTree::new();
        let ui = crate::MelDocument::parse_with(
            &meui,
            &std::fs::read_to_string(&meui).unwrap(),
            &crate::source_dir::read_sets(&[dir.join("Strings.mres")]).unwrap(),
        )
        .unwrap()
        .ui
        .unwrap();
        let root = build(&mut tree, &ui);
        let document = HotDocument::open(&meui, Some(dir), root, DEBOUNCE).unwrap();
        (tree, root, document)
    }

    #[test]
    fn a_changed_resource_patches_the_tree() {
        let dir = scratch("patch");
        let (mut tree, root, mut document) = open(&dir);
        let label = tree.children(root).unwrap()[0];
        let start = Instant::now();
        assert!(document.refresh(&mut tree, start).unwrap().is_none());
        write(&dir.join("Strings.mres"), r#"{ title: "Meridian" }"#);
        document.refresh(&mut tree, start).unwrap();
        let refreshed = document.refresh(&mut tree, start + DEBOUNCE * 2).unwrap();
        assert_eq!(refreshed, Some(Refreshed::Patched { changed: 1 }));
        let text = tree.get(label, properties::text_block::TEXT).unwrap();
        assert_eq!(text, "Meridian");
    }

    #[test]
    fn a_new_shape_rebuilds_and_a_bad_file_keeps_the_tree() {
        let dir = scratch("rebuild");
        let (mut tree, root, mut document) = open(&dir);
        let start = Instant::now();
        let meui = dir.join("hud.meui");
        write(&meui, "<StackPanel><TextBlock");
        document.refresh(&mut tree, start).unwrap();
        let error = document
            .refresh(&mut tree, start + DEBOUNCE * 2)
            .unwrap_err();
        assert!(matches!(error, ReloadError::Build(_)), "{error}");
        assert!(tree.contains(root));
        write(
            &meui,
            r#"<StackPanel><TextBlock x:Name="a" /><TextBlock x:Name="b" /></StackPanel>"#,
        );
        let later = start + DEBOUNCE * 4;
        document.refresh(&mut tree, later).unwrap();
        let refreshed = document.refresh(&mut tree, later + DEBOUNCE * 2).unwrap();
        let Some(Refreshed::Rebuilt(rebuilt)) = refreshed else {
            panic!("expected a rebuild, got {refreshed:?}");
        };
        assert!(!tree.contains(root));
        assert_eq!(tree.children(rebuilt.root).unwrap().len(), 2);
        assert_eq!(
            rebuilt.node("b"),
            Some(tree.children(rebuilt.root).unwrap()[1])
        );
    }

    #[test]
    fn switching_the_culture_patches_without_a_file_change() {
        let dir = scratch("culture");
        let (mut tree, root, mut document) = open(&dir);
        write(&dir.join("Strings.ru.mres"), r#"{ title: "Йог" }"#);
        let label = tree.children(root).unwrap()[0];
        let start = Instant::now();
        let ru = crate::Culture::parse("ru");
        document.set_culture(ru.clone());
        let refreshed = document.refresh(&mut tree, start).unwrap();
        assert_eq!(refreshed, Some(Refreshed::Patched { changed: 1 }));
        assert_eq!(
            tree.get(label, properties::text_block::TEXT).unwrap(),
            "Йог"
        );
        document.set_culture(ru);
        assert_eq!(document.refresh(&mut tree, start).unwrap(), None);
        document.set_culture(None);
        document.refresh(&mut tree, start).unwrap();
        assert_eq!(
            tree.get(label, properties::text_block::TEXT).unwrap(),
            "Yog"
        );
    }
}

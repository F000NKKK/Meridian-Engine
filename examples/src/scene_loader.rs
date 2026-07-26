//! Reading a `.mel` scene file into a built `meridian_sdk::dsl` tree —
//! the read-file/parse/build sequence was identical between
//! `physic_figures`/`magic_figures`; only the registry (which tags are
//! known) and what each example does with the resulting tree differ.

use meridian_sdk::dsl::build_scene;
use meridian_sdk::dsl::dsl_core::{BuiltNode, DslRegistry};

use crate::paths::asset_path;

/// Reads `relative_path` (joined via [`asset_path`]) and builds it
/// against `registry` in one call. A missing file or a malformed
/// document logs via `log_error!` before panicking — every DSL error
/// implements `meridian_foundation::EngineError` (see
/// `meridian_sdk::dsl`'s own module doc), so `crash_reporting`'s
/// post-mortem still captures exactly what went wrong; this isn't a
/// silent dead end, just a real one an example can't meaningfully
/// recover from (there is no scene to fall back to).
pub fn load_dsl_scene(relative_path: &str, registry: &DslRegistry) -> BuiltNode {
    let path = asset_path(relative_path);
    let source = std::fs::read_to_string(&path).unwrap_or_else(|e| {
        meridian_sdk::log_error!("failed to read scene {path}: {e}");
        panic!("failed to read scene {path}: {e}");
    });
    build_scene(&source, registry).unwrap_or_else(|e| {
        meridian_sdk::log_error!("failed to parse scene {path}: {e}");
        panic!("failed to parse scene {path}: {e}");
    })
}

/// The first child of `node` buildable as `T` — a convenience over
/// repeating `node.children.iter().find_map(|c| c.downcast_ref::<T>())`
/// at every call site that just wants "does this entity have a
/// `<Transform>`/`<Mesh>`/etc." without caring which position it's in.
pub fn find_child<T: 'static>(node: &BuiltNode) -> Option<&T> {
    node.children
        .iter()
        .find_map(|child| child.downcast_ref::<T>())
}

#[cfg(test)]
mod tests {
    use super::*;
    use meridian_sdk::dsl::{DslTag, TagParseError, dsl_tag};

    #[dsl_tag(name = "Widget")]
    #[derive(Debug, PartialEq)]
    struct Widget {
        hp: f32,
    }

    #[test]
    fn find_child_locates_the_matching_downcast() {
        let mut registry = DslRegistry::new();
        registry.register::<meridian_sdk::dsl::Entity>();
        registry.register::<Widget>();
        let root = build_scene(
            r#"<Entity name="thing"><Widget hp="12.5" /></Entity>"#,
            &registry,
        )
        .unwrap();
        assert_eq!(find_child::<Widget>(&root), Some(&Widget { hp: 12.5 }));
    }

    #[test]
    fn find_child_returns_none_when_absent() {
        let mut registry = DslRegistry::new();
        registry.register::<meridian_sdk::dsl::Entity>();
        let root = build_scene(r#"<Entity name="thing" />"#, &registry).unwrap();
        assert_eq!(find_child::<Widget>(&root), None);
    }
}

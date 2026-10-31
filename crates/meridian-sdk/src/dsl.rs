//! Extensible scene DSL: a small XAML-flavored tag language for
//! describing entity composition, where a game developer registers
//! their *own* tags rather than picking from a fixed schema this crate
//! hardcodes. This directly answers the concrete correction that
//! replaced this module's first (rejected) design: there is no
//! `<Entity><Mesh><Material><RigidBody>` schema baked in here — this
//! file defines a handful of built-in tags the same way a game's own
//! crate would define its own with `#[dsl_tag]`, and nothing about
//! [`DslRegistry`]/[`build_scene`] privileges them.
//!
//! ## The three layers
//!
//! Design notes: docs/notes/meridian-sdk.md (`dsl`).

use dsl_core::{DslRegistry, TagParseError};
/// The domain-blind parser and typed-tag machinery — see the module doc
/// above for how this layers under [`dsl_tag`] and this module's
/// built-in tags.
pub use meridian_markup as dsl_core;
pub use meridian_markup_macros::dsl_tag;

/// The document root — a plain container for [`Entity`] nodes, carrying
/// no data of its own. Every DSL document this crate writes has one of
/// these at the top; an application walking the built tree only cares
/// about its `children`.
#[dsl_tag(name = "Scene")]
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Scene {}

/// A named node in the scene tree — the root composition primitive;
/// almost every DSL document's root and internal nodes are this tag,
/// with type-specific tags (`Mesh`, `RigidBody`, ...) as children
/// describing what the entity *has*.
#[dsl_tag(name = "Entity")]
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    /// Name used to find this entity in the built scene tree.
    pub name: String,
}

/// A procedural or file-loaded mesh reference. `path`, when present,
/// names a file an application loads via
/// [`crate::assets::AssetCache::load_mesh_obj`]; `shape`, when present,
/// names one of this crate's own procedural builders
/// (`"cube"`/`"sphere"`/`"pyramid"`/`"ground"`), with `size`/`size2` as
/// that builder's scale parameters (one shape, one meaning each:
/// `"cube"` uses `size` as its half-extent; `"sphere"` uses `size` as
/// its radius; `"pyramid"` uses `size` as its base half-extent and
/// `size2` as its height; `"ground"` uses `size` as its half-extent and
/// `size2` as its UV tile count) — an application walking the built
/// tree decides which of `path`/`shape` (or both, for its own custom
/// shapes) it honors, this tag only carries the data.
#[dsl_tag(name = "Mesh")]
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    /// Optional OBJ file loaded by the application.
    pub path: Option<String>,
    /// Optional procedural shape name.
    pub shape: Option<String>,
    /// Primary shape size parameter.
    pub size: Option<f32>,
    /// Secondary shape size parameter.
    pub size2: Option<f32>,
}

/// A material reference by texture file path plus a uniform base-color
/// tint — the common case; a material needing more (custom shaders,
/// multiple textures) is exactly the kind of thing a game defines its
/// own `#[dsl_tag]` for instead of stretching this one.
#[dsl_tag(name = "Material")]
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    /// Optional albedo texture path.
    pub texture: Option<String>,
    /// Whether to bypass scene lighting.
    pub unlit: Option<bool>,
}

/// A rigid-body collider description — `shape` is `"sphere"` (using
/// `radius`) or `"cuboid"` (using `hx`/`hy`/`hz`, one half-extent per
/// axis — a floor slab and a cube prop need different aspect ratios, so
/// this doesn't collapse to a single uniform `size` the way procedural
/// `Mesh` shapes mostly do), `mass` of `0.0` meaning static/immovable,
/// matching `physics-domain::RigidBody`'s own convention.
#[dsl_tag(name = "RigidBody")]
#[derive(Debug, Clone, PartialEq)]
pub struct RigidBody {
    /// Collider kind: `sphere` or `cuboid`.
    pub shape: String,
    /// Mass in kilograms; zero means static.
    pub mass: f32,
    /// Sphere radius when `shape` is `sphere`.
    pub radius: Option<f32>,
    /// Cuboid half-extent along X.
    pub hx: Option<f32>,
    /// Cuboid half-extent along Y.
    pub hy: Option<f32>,
    /// Cuboid half-extent along Z.
    pub hz: Option<f32>,
}

/// World placement: translation only (rotation/scale composition
/// through text attributes gets unreadable fast — a DSL document that
/// needs a starting rotation is exactly the case for building that one
/// entity's frame in Rust and only using the DSL for the rest).
#[dsl_tag(name = "Transform")]
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    /// Translation along X.
    pub x: f32,
    /// Translation along Y.
    pub y: f32,
    /// Translation along Z.
    pub z: f32,
}

/// A [`DslRegistry`] with this module's built-in tags
/// ([`Entity`]/[`Mesh`]/[`Material`]/[`RigidBody`]/[`Transform`])
/// already registered — the common starting point for an application
/// that wants those plus its own custom tags:
///
/// ```
/// # use meridian_sdk::dsl::default_registry;
/// let mut registry = default_registry();
/// // registry.register::<MyGameSpecificWidget>();
/// ```
#[must_use]
pub fn default_registry() -> DslRegistry {
    let mut registry = DslRegistry::new();
    registry.register::<Scene>();
    registry.register::<Entity>();
    registry.register::<Mesh>();
    registry.register::<Material>();
    registry.register::<RigidBody>();
    registry.register::<Transform>();
    registry
}

/// Parses `source` and builds it against `registry` in one call — the
/// common path; an application that wants the intermediate
/// [`dsl_core::Element`] tree (to validate structure before building,
/// say) calls [`dsl_core::parse`] and [`DslRegistry::build`] itself
/// instead.
pub fn build_scene(source: &str, registry: &DslRegistry) -> Result<dsl_core::BuiltNode, DslError> {
    let element = dsl_core::parse(source).map_err(DslError::Parse)?;
    registry.build(&element).map_err(DslError::Tag)
}

/// Pulls in the module `meridian-ui-mel` generated for a `.meui`
/// document (ADR 017): `meui_include!("magic_figures")` includes
/// `$OUT_DIR/meui/magic_figures.g.rs`, written by
/// `meridian_ui_mel::compile_dir` in the crate's `build.rs`. Use it
/// inside the code-behind module:
///
/// ```ignore
/// mod scene {
///     meridian_sdk::meui_include!("magic_figures");
/// }
/// let root = scene::build(&registry)?;
/// let cube = scene::find(&root, scene::Name::Cube);
/// ```
#[macro_export]
macro_rules! meui_include {
    ($name:literal) => {
        include!(concat!(env!("OUT_DIR"), "/meui/", $name, ".g.rs"));
    };
}

/// Either half of what can go wrong turning DSL text into a built tree
/// — kept as one type so a caller doesn't need two separate `match`
/// arms for what is, to it, one "the document was bad" outcome.
#[derive(Debug)]
pub enum DslError {
    /// The document's markup syntax is invalid.
    Parse(dsl_core::ParseError),
    /// A tag could not construct its typed value.
    Tag(TagParseError),
}

impl std::fmt::Display for DslError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DslError::Parse(e) => write!(f, "DSL parse error {e}"),
            DslError::Tag(e) => write!(f, "DSL build error: {e}"),
        }
    }
}

impl std::error::Error for DslError {}

impl meridian_diagnostics::EngineError for DslError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_tags_parse_a_small_scene() {
        let src = r#"
            <Scene>
                <Entity name="cube">
                    <Transform x="0" y="2" z="0" />
                    <Mesh shape="cube" size="1.0" />
                    <Material texture="cube.bmp" unlit="false" />
                    <RigidBody shape="cuboid" mass="1.0" hx="0.6" hy="0.6" hz="0.6" />
                </Entity>
            </Scene>
        "#;
        let root = build_scene(src, &default_registry()).unwrap();
        assert_eq!(root.children.len(), 1);
        let node = &root.children[0];
        assert_eq!(node.downcast_ref::<Entity>().unwrap().name, "cube");
        assert_eq!(node.children.len(), 4);
        assert_eq!(
            node.children[0].downcast_ref::<Transform>().unwrap(),
            &Transform {
                x: 0.0,
                y: 2.0,
                z: 0.0
            }
        );
        let mesh = node.children[1].downcast_ref::<Mesh>().unwrap();
        assert_eq!(mesh.shape.as_deref(), Some("cube"));
        assert_eq!(mesh.path, None);
        let body = node.children[3].downcast_ref::<RigidBody>().unwrap();
        assert_eq!(body.mass, 1.0);
        assert_eq!(body.hx, Some(0.6));
    }

    #[test]
    fn custom_game_tag_composes_with_built_ins() {
        #[dsl_tag(name = "Health")]
        #[derive(Debug, PartialEq)]
        struct Health {
            hp: f32,
        }

        let mut registry = default_registry();
        registry.register::<Health>();

        let src = r#"<Entity name="player"><Health hp="100" /></Entity>"#;
        let node = build_scene(src, &registry).unwrap();
        // Not wrapped in `<Scene>` here — proves `build_scene` doesn't
        // require the root to be that specific tag, only that it and
        // everything below it is registered.
        assert_eq!(
            node.children[0].downcast_ref::<Health>(),
            Some(&Health { hp: 100.0 })
        );
    }

    #[test]
    fn missing_required_attribute_surfaces_as_dsl_error() {
        let err = build_scene(r#"<RigidBody shape="sphere" />"#, &default_registry()).unwrap_err();
        assert!(matches!(err, DslError::Tag(_)));
        assert!(err.to_string().contains("mass"));
    }
}

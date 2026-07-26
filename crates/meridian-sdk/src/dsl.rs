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
//! - [`meridian_dsl_core`] (re-exported here as [`dsl_core`]) — the
//!   domain-blind text parser (`<Tag attr="value">...</Tag>` ->
//!   `Element`) plus [`dsl_core::DslTag`]/[`dsl_core::DslRegistry`], the
//!   generic "typed thing buildable from attributes" machinery. Never
//!   changes when a new tag is added.
//! - [`dsl_tag`] (the `#[dsl_tag(name = "...")]` attribute macro,
//!   re-exported from `meridian-dsl-macros`) — code generation: turn a
//!   plain struct into a [`dsl_core::DslTag`] impl, one required
//!   attribute per field (`Option<T>` fields are optional). This is how
//!   a game adds `<MyGameSpecificWidget hp="100" />` without touching
//!   this crate at all.
//! - This module's own [`Entity`]/[`Mesh`]/[`Material`]/[`RigidBody`]/
//!   [`Transform`] — built-in tags for the composition primitives every
//!   application needs, registered by [`default_registry`]. An
//!   application that doesn't want one of these can build its own
//!   `DslRegistry` from scratch instead of calling [`default_registry`].
//!
//! ## Scope, deliberately
//!
//! This DSL describes scene/entity composition only — no window title,
//! no logging config, no crash-report directory (see the earlier design
//! discussion that scoped this to Phase 1). An application still writes
//! that part in plain Rust `main()`, same as today.
//!
//! Parse errors ([`dsl_core::ParseError`]) and tag-build errors
//! ([`dsl_core::TagParseError`]) both implement
//! `meridian_foundation::EngineError` (see that trait's own doc) — a
//! caller that lets one propagate into a panic (rather than, say,
//! falling back to an empty scene) gets it captured by
//! `crash_reporting`'s post-mortem for free, same as every other engine
//! error; nothing about this module is a silent dead end.

/// The domain-blind parser and typed-tag machinery — see the module doc
/// above for how this layers under [`dsl_tag`] and this module's
/// built-in tags.
pub use meridian_dsl_core as dsl_core;
pub use meridian_dsl_macros::dsl_tag;

use dsl_core::{DslRegistry, TagParseError};

/// A named node in the scene tree — the root composition primitive;
/// almost every DSL document's root and internal nodes are this tag,
/// with type-specific tags (`Mesh`, `RigidBody`, ...) as children
/// describing what the entity *has*.
#[dsl_tag(name = "Entity")]
#[derive(Debug, Clone, PartialEq)]
pub struct Entity {
    pub name: String,
}

/// A procedural or file-loaded mesh reference. `path`, when present,
/// names a file an application loads via
/// [`crate::assets::AssetCache::load_mesh_obj`]; `shape`, when present,
/// names one of this crate's own procedural builders
/// (`"cube"`/`"sphere"`/`"pyramid"`/`"ground"`) with `size` as that
/// builder's single scale parameter — an application walking the built
/// tree decides which of the two (or both, for its own custom shapes)
/// it honors, this tag only carries the data.
#[dsl_tag(name = "Mesh")]
#[derive(Debug, Clone, PartialEq)]
pub struct Mesh {
    pub path: Option<String>,
    pub shape: Option<String>,
    pub size: Option<f32>,
}

/// A material reference by texture file path plus a uniform base-color
/// tint — the common case; a material needing more (custom shaders,
/// multiple textures) is exactly the kind of thing a game defines its
/// own `#[dsl_tag]` for instead of stretching this one.
#[dsl_tag(name = "Material")]
#[derive(Debug, Clone, PartialEq)]
pub struct Material {
    pub texture: Option<String>,
    pub unlit: Option<bool>,
}

/// A rigid-body collider description — `shape` is `"sphere"` (using
/// `radius`) or `"cuboid"` (using `half_extent`, applied to all three
/// axes; a per-axis cuboid is exactly the kind of shape-specific detail
/// a game's own tag would carry instead), `mass` of `0.0` meaning
/// static/immovable, matching `physics-core::RigidBody`'s own
/// convention.
#[dsl_tag(name = "RigidBody")]
#[derive(Debug, Clone, PartialEq)]
pub struct RigidBody {
    pub shape: String,
    pub mass: f32,
    pub radius: Option<f32>,
    pub half_extent: Option<f32>,
}

/// World placement: translation only (rotation/scale composition
/// through text attributes gets unreadable fast — a DSL document that
/// needs a starting rotation is exactly the case for building that one
/// entity's frame in Rust and only using the DSL for the rest).
#[dsl_tag(name = "Transform")]
#[derive(Debug, Clone, PartialEq)]
pub struct Transform {
    pub x: f32,
    pub y: f32,
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
pub fn default_registry() -> DslRegistry {
    let mut registry = DslRegistry::new();
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

/// Either half of what can go wrong turning DSL text into a built tree
/// — kept as one type so a caller doesn't need two separate `match`
/// arms for what is, to it, one "the document was bad" outcome.
#[derive(Debug)]
pub enum DslError {
    Parse(dsl_core::ParseError),
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

impl meridian_foundation::EngineError for DslError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_tags_parse_a_small_scene() {
        let src = r#"
            <Entity name="cube">
                <Transform x="0" y="2" z="0" />
                <Mesh shape="cube" size="1.0" />
                <Material texture="cube.bmp" unlit="false" />
                <RigidBody shape="cuboid" mass="1.0" half_extent="0.6" />
            </Entity>
        "#;
        let node = build_scene(src, &default_registry()).unwrap();
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
        assert_eq!(body.half_extent, Some(0.6));
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

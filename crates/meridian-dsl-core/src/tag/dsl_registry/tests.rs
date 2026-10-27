//! Tests for [`DslRegistry`] with hand-written [`DslTag`] impls.

use super::*;
use crate::{parse, text_attr};

#[derive(Debug, PartialEq)]
struct RigidBodyTag {
    mass: f32,
}

impl DslTag for RigidBodyTag {
    const TAG_NAME: &'static str = "RigidBody";

    fn from_attrs(attrs: &[Attribute]) -> Result<Self, TagParseError> {
        let mass = text_attr(attrs, Self::TAG_NAME, "mass")?
            .ok_or_else(|| TagParseError {
                message: "missing attribute 'mass'".to_string(),
            })?
            .parse::<f32>()
            .map_err(|e| TagParseError {
                message: format!("invalid 'mass': {e}"),
            })?;
        Ok(Self { mass })
    }
}

#[test]
fn registered_tag_builds_and_downcasts() {
    let mut registry = DslRegistry::new();
    registry.register::<RigidBodyTag>();
    let element = parse(r#"<RigidBody mass="2.5" />"#).unwrap();
    let node = registry.build(&element).unwrap();
    assert_eq!(node.tag, "RigidBody");
    assert_eq!(
        node.downcast_ref::<RigidBodyTag>(),
        Some(&RigidBodyTag { mass: 2.5 })
    );
}

#[test]
fn unregistered_tag_is_an_error_not_a_silent_skip() {
    let mut registry = DslRegistry::new();
    registry.register::<RigidBodyTag>();
    let element = parse(r#"<TotallyMadeUpTag />"#).unwrap();
    let err = registry.build(&element).unwrap_err();
    assert!(err.message.contains("unknown DSL tag"));
}

#[test]
fn missing_required_attribute_is_an_error() {
    let mut registry = DslRegistry::new();
    registry.register::<RigidBodyTag>();
    let element = parse(r#"<RigidBody />"#).unwrap();
    let err = registry.build(&element).unwrap_err();
    assert!(err.message.contains("missing attribute 'mass'"));
}

#[test]
fn markup_extension_where_text_is_expected_is_an_error() {
    let mut registry = DslRegistry::new();
    registry.register::<RigidBodyTag>();
    let element = parse(r#"<RigidBody mass="{x:Bind mass}" />"#).unwrap();
    let err = registry.build(&element).unwrap_err();
    assert!(err.message.contains("markup extension"));
}

#[test]
fn nested_custom_tags_all_build() {
    #[derive(Debug, PartialEq)]
    struct EntityTag {
        name: String,
    }
    impl DslTag for EntityTag {
        const TAG_NAME: &'static str = "Entity";
        fn from_attrs(attrs: &[Attribute]) -> Result<Self, TagParseError> {
            let name = text_attr(attrs, Self::TAG_NAME, "name")?
                .ok_or_else(|| TagParseError {
                    message: "missing attribute 'name'".to_string(),
                })?
                .to_string();
            Ok(Self { name })
        }
    }

    let mut registry = DslRegistry::new();
    registry.register::<EntityTag>();
    registry.register::<RigidBodyTag>();
    let element = parse(r#"<Entity name="crate"><RigidBody mass="1.0" /></Entity>"#).unwrap();
    let node = registry.build(&element).unwrap();
    assert_eq!(
        node.downcast_ref::<EntityTag>(),
        Some(&EntityTag {
            name: "crate".to_string()
        })
    );
    assert_eq!(node.children.len(), 1);
    assert_eq!(
        node.children[0].downcast_ref::<RigidBodyTag>(),
        Some(&RigidBodyTag { mass: 1.0 })
    );
}

#[test]
fn x_name_is_kept_and_found() {
    let mut registry = DslRegistry::new();
    registry.register::<RigidBodyTag>();
    let element = parse(r#"<RigidBody x:Name="crate" mass="1.0" />"#).unwrap();
    let node = registry.build(&element).unwrap();
    assert_eq!(node.name.as_deref(), Some("crate"));
    assert!(node.find_named("crate").is_some());
    assert!(node.find_named("missing").is_none());
}

//! Tests of this module.

use super::*;

#[test]
fn self_closing_element_with_attrs() {
    let root = parse(r#"<Mesh path="cube.obj" scale="2" />"#).unwrap();
    assert_eq!(root.tag, "Mesh");
    assert_eq!(root.attr("path"), Some("cube.obj"));
    assert_eq!(root.attr("scale"), Some("2"));
    assert!(root.children.is_empty());
}

#[test]
fn nested_children_in_source_order() {
    let src = r#"
        <Entity name="cube">
            <Mesh path="cube.obj" />
            <RigidBody mass="1.0" />
        </Entity>
    "#;
    let root = parse(src).unwrap();
    assert_eq!(root.tag, "Entity");
    assert_eq!(root.attr("name"), Some("cube"));
    assert_eq!(root.children.len(), 2);
    assert_eq!(root.children[0].tag, "Mesh");
    assert_eq!(root.children[1].tag, "RigidBody");
}

#[test]
fn escaped_quote_in_attribute_value() {
    let root = parse(r#"<Label text="say \"hi\"" />"#).unwrap();
    assert_eq!(root.attr("text"), Some("say \"hi\""));
}

#[test]
fn mismatched_closing_tag_is_an_error() {
    let err = parse("<A><B></C></A>").unwrap_err();
    assert!(err.message.contains("mismatched closing tag"));
}

#[test]
fn duplicate_attribute_is_an_error() {
    let err = parse(r#"<Mesh path="a.obj" path="b.obj" />"#).unwrap_err();
    assert!(err.message.contains("duplicate attribute"));
}

#[test]
fn unrecognized_tag_names_parse_fine_here() {
    // Proves the parser is domain-blind: an entirely made-up tag
    // parses identically to a built-in-looking one, since typed
    // meaning is added one layer up, not by this crate.
    let root = parse(r#"<MyGameSpecificWidget hp="100" />"#).unwrap();
    assert_eq!(root.tag, "MyGameSpecificWidget");
    assert_eq!(root.attr("hp"), Some("100"));
}

#[test]
fn trailing_garbage_after_root_is_an_error() {
    let err = parse(r#"<A /> <B />"#).unwrap_err();
    assert!(err.message.contains("end of document"));
}

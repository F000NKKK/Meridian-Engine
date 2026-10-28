//! Tests for [`parse`]: structure, names, values, spans and errors.

use super::*;
use crate::Span;

fn kind(source: &str) -> ParseErrorKind {
    parse(source).unwrap_err().kind
}

fn text<'a>(element: &'a Element, name: &str) -> Option<&'a str> {
    element.attr(name).and_then(Attribute::text)
}

#[test]
fn self_closing_element_with_attrs() {
    let root = parse(r#"<Mesh path="cube.obj" scale="2" />"#).unwrap();
    assert!(root.name.is_plain("Mesh"));
    assert_eq!(text(&root, "path"), Some("cube.obj"));
    assert_eq!(text(&root, "scale"), Some("2"));
    assert_eq!(root.children, Vec::new());
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
    assert!(root.name.is_plain("Entity"));
    assert_eq!(text(&root, "name"), Some("cube"));
    assert_eq!(root.children.len(), 2);
    assert!(root.children[0].name.is_plain("Mesh"));
    assert!(root.children[1].name.is_plain("RigidBody"));
}

#[test]
fn spans_point_at_elements_and_attributes() {
    let root = parse("<Scene>\n  <Entity\n     name=\"a\" />\n</Scene>").unwrap();
    assert_eq!(
        root.span,
        Span {
            offset: 0,
            line: 1,
            column: 1
        }
    );
    let entity = &root.children[0];
    assert_eq!((entity.span.line, entity.span.column), (2, 3));
    let name = &entity.attrs[0].span;
    assert_eq!((name.line, name.column), (3, 6));
}

#[test]
fn error_span_has_line_and_column() {
    let err = parse("<A>\n  <B></C>\n</A>").unwrap_err();
    assert_eq!((err.span.line, err.span.column), (2, 8));
    assert_eq!(err.to_string().split(':').next(), Some("2"));
}

#[test]
fn escaped_quote_in_attribute_value() {
    let root = parse(r#"<Label text="say \"hi\"" />"#).unwrap();
    assert_eq!(text(&root, "text"), Some("say \"hi\""));
}

#[test]
fn directives_and_attached_properties() {
    let root = parse(r#"<Button x:Name="ok" Grid.Row="1" text="OK" />"#).unwrap();
    let directive = &root.attrs[0].name;
    assert_eq!(directive.prefix(), Some(crate::Prefix::Directive));
    assert_eq!(directive.local(), "Name");
    assert_eq!(root.attrs[1].name.owner(), Some("Grid"));
    assert_eq!(text(&root, "text"), Some("OK"));
    assert_eq!(text(&root, "Name"), None);
}

#[test]
fn markup_extension_value_is_parsed() {
    let root = parse(r#"<Label text="{x:Bind count_text()}" />"#).unwrap();
    let Some(MarkupValue::Extension(bind)) = root.attr("text").map(|a| &a.value) else {
        panic!("expected a markup extension");
    };
    assert_eq!(bind.name.to_string(), "x:Bind");
}

#[test]
fn property_element_inside_its_owner() {
    let root = parse("<Entity><Entity.Components><Mesh /></Entity.Components></Entity>").unwrap();
    let property = &root.children[0];
    assert!(property.is_property_element());
    assert_eq!(property.children.len(), 1);
}

#[test]
fn misplaced_property_elements_are_errors() {
    assert!(matches!(
        kind("<A.B />"),
        ParseErrorKind::PropertyElementAtRoot(_)
    ));
    assert!(matches!(
        kind("<Entity><Grid.Row /></Entity>"),
        ParseErrorKind::PropertyElementOwner { .. }
    ));
    assert!(matches!(
        kind(r#"<Entity><Entity.Components a="1" /></Entity>"#),
        ParseErrorKind::PropertyElementAttributes(_)
    ));
}

#[test]
fn prefix_errors() {
    assert_eq!(
        kind(r#"<A y:Name="a" />"#),
        ParseErrorKind::UnknownPrefix("y".to_string())
    );
    assert!(matches!(
        kind("<x:Code />"),
        ParseErrorKind::DirectiveElement(_)
    ));
    assert_eq!(kind(r#"<A b.="1" />"#), ParseErrorKind::ExpectedIdentifier);
}

#[test]
fn malformed_extension_is_an_error_at_its_position() {
    assert!(matches!(
        kind(r#"<A b="{x:Bind" />"#),
        ParseErrorKind::Extension { .. }
    ));
    let err = parse(r#"<A b="{x:Bind" />"#).unwrap_err();
    assert_eq!(err.span.column, 14);
    // An escaped quote is two source bytes for one value byte.
    let err = parse(r#"<A b="{x:Bind \"a" />"#).unwrap_err();
    assert_eq!(err.span.column, 18);
}

#[test]
fn structural_errors() {
    assert!(matches!(
        kind("<A><B></C></A>"),
        ParseErrorKind::MismatchedClosingTag { .. }
    ));
    assert_eq!(
        kind(r#"<Mesh path="a.obj" path="b.obj" />"#),
        ParseErrorKind::DuplicateAttribute("path".to_string())
    );
    assert_eq!(kind("<A /> <B />"), ParseErrorKind::TrailingContent);
    assert!(matches!(
        kind("<A>"),
        ParseErrorKind::UnterminatedElement(_)
    ));
    assert_eq!(kind(r#"<A b="x"#), ParseErrorKind::UnterminatedString);
    assert_eq!(kind(r#"<A b="\n" />"#), ParseErrorKind::InvalidEscape);
    assert_eq!(kind("<A b=1 />"), ParseErrorKind::Expected('"'));
    assert_eq!(kind(""), ParseErrorKind::Expected('<'));
}

#[test]
fn unrecognized_tag_names_parse_fine_here() {
    // The parser is domain-blind: a made-up tag parses identically to a
    // built-in-looking one; typed meaning is added one layer up.
    let root = parse(r#"<MyGameSpecificWidget hp="100" />"#).unwrap();
    assert!(root.name.is_plain("MyGameSpecificWidget"));
    assert_eq!(text(&root, "hp"), Some("100"));
}

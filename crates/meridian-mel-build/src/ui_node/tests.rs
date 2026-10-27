//! Tests for [`UiNode::from_element`].

use meridian_dsl_core::parse;

use super::*;

fn node(source: &str) -> Result<UiNode, InvalidMarkup> {
    let root = parse(source).unwrap();
    let span = Span {
        offset: 0,
        line: 1,
        column: 1,
    };
    let names = [NamedElement::new("title", span).unwrap()];
    UiNode::from_element(&root, &names, &ResourceSets::default()).map_err(|(_, reason)| reason)
}

#[test]
fn builds_kinds_fields_literals_and_children() {
    let root = node(
        r#"<StackPanel width="200" Grid.Row="1" x:Name="title">
            <TextBlock text="Hi" font_size="{x:Bind size}" />
        </StackPanel>"#,
    )
    .unwrap();
    assert_eq!(root.kind, ElementKind::StackPanel);
    assert_eq!(root.field.as_deref(), Some("title"));
    let literals: Vec<_> = root
        .literals
        .iter()
        .map(|l| (l.property.as_str(), l.text.as_str()))
        .collect();
    assert_eq!(literals, [("width", "200"), ("Grid.Row", "1")]);
    let child = &root.children[0];
    assert_eq!(child.kind, ElementKind::TextBlock);
    assert_eq!(child.field, None);
    // The bound attribute is not a literal.
    assert_eq!(child.literals.len(), 1);
}

#[test]
fn rejects_what_the_tree_cannot_build() {
    assert_eq!(
        node("<StackPanel><Sprite /></StackPanel>"),
        Err(InvalidMarkup::UnknownElement("Sprite".to_string()))
    );
    assert_eq!(
        node("<Grid><Grid.RowDefinitions /></Grid>"),
        Err(InvalidMarkup::UnknownElement(
            "Grid.RowDefinitions".to_string()
        ))
    );
    assert_eq!(
        node(r#"<TextBlock text="{StaticResource K}" />"#),
        Err(InvalidMarkup::UnsupportedExtension(
            "StaticResource".to_string()
        ))
    );
    assert!(matches!(
        node(r#"<TextBlock colour="red" />"#),
        Err(InvalidMarkup::UnknownProperty { .. })
    ));
}

#[test]
fn ui_roots_are_recognised_by_tag() {
    assert!(UiNode::is_ui_root(&parse("<Grid />").unwrap()));
    assert!(!UiNode::is_ui_root(&parse("<Scene />").unwrap()));
}

#[test]
fn resource_references_are_resolved_in_attributes() {
    let mut resources = ResourceSets::default();
    let set = crate::ResourceSet::parse(
        std::path::Path::new("Strings.mres"),
        "Strings",
        r#"{ title: "Yog" }"#,
    )
    .unwrap();
    resources.insert(set);
    let root = parse(r#"<TextBlock text="@res:Strings.title" />"#).unwrap();
    let node = UiNode::from_element(&root, &[], &resources).unwrap();
    assert_eq!(node.literals[0].text, "Yog");
    let missing = parse(r#"<TextBlock text="@res:Strings.nope" />"#).unwrap();
    assert_eq!(
        UiNode::from_element(&missing, &[], &resources)
            .unwrap_err()
            .1,
        InvalidMarkup::UnknownResource("Strings.nope".to_string())
    );
}

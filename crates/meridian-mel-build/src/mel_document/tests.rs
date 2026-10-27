//! Tests for [`MelDocument::parse`]: what it collects and what it rejects.

use super::*;

fn document(source: &str) -> Result<MelDocument, BuildError> {
    MelDocument::parse(Path::new("scene.mel"), source)
}

fn rejection(source: &str) -> InvalidMarkup {
    match document(source) {
        Err(BuildError::Invalid { reason, .. }) => reason,
        other => panic!("expected invalid markup, got {other:?}"),
    }
}

#[test]
fn collects_names_and_code_behind_types() {
    let doc = document(
        r#"<Scene x:Class="crate::scene::Magic" x:ViewModel="Vm">
            <Entity x:Name="cube" />
            <Entity><Mesh x:Name="left_arm" /></Entity>
        </Scene>"#,
    )
    .unwrap();
    assert_eq!(doc.class.as_deref(), Some("crate::scene::Magic"));
    assert_eq!(doc.view_model.as_deref(), Some("Vm"));
    let variants: Vec<_> = doc.names.iter().map(|n| n.variant.as_str()).collect();
    assert_eq!(variants, ["Cube", "LeftArm"]);
    assert_eq!(doc.names[1].span.line, 3);
}

#[test]
fn plain_document_has_nothing_extra() {
    let doc = document(r#"<Scene><Entity name="a" /></Scene>"#).unwrap();
    assert!(doc.names.is_empty() && doc.class.is_none() && doc.view_model.is_none());
}

#[test]
fn directive_rules() {
    assert_eq!(
        rejection(r#"<A x:Code="x" />"#),
        InvalidMarkup::UnknownDirective("Code".to_string())
    );
    assert_eq!(
        rejection(r#"<A><B x:Class="T" /></A>"#),
        InvalidMarkup::RootOnlyDirective("Class".to_string())
    );
    assert_eq!(
        rejection(r#"<A x:Name="{x:Bind n}" />"#),
        InvalidMarkup::DirectiveNotText("Name".to_string())
    );
    assert_eq!(
        rejection(r#"<A x:Class="crate::not a type" />"#),
        InvalidMarkup::InvalidTypePath("crate::not a type".to_string())
    );
}

#[test]
fn name_rules() {
    assert_eq!(
        rejection(r#"<A x:Name="2d" />"#),
        InvalidMarkup::InvalidName("2d".to_string())
    );
    assert_eq!(
        rejection(r#"<A><B x:Name="a" /><C x:Name="a" /></A>"#),
        InvalidMarkup::DuplicateName("a".to_string())
    );
    assert_eq!(
        rejection(r#"<A><B x:Name="left_arm" /><C x:Name="left-arm" /></A>"#),
        InvalidMarkup::NameCollision {
            first: "left_arm".to_string(),
            second: "left-arm".to_string()
        }
    );
}

#[test]
fn parse_errors_keep_the_path() {
    let error = document("<A>").unwrap_err();
    assert!(error.to_string().starts_with("scene.mel:1:"), "{error}");
}

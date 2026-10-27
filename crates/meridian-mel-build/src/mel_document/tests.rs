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

#[test]
fn collects_bindings_on_named_elements() {
    let doc = document(
        r#"<StackPanel x:ViewModel="Vm">
            <TextBlock x:Name="score_label" text="{x:Bind score_text(), Mode=OneTime}" Grid.Row="{x:Bind row}" />
        </StackPanel>"#,
    )
    .unwrap();
    let bound: Vec<_> = doc
        .bindings
        .iter()
        .map(|b| (b.field.as_str(), b.property.as_str(), b.mode))
        .collect();
    assert_eq!(
        bound,
        [
            ("score_label", "text", crate::BindingMode::OneTime),
            ("score_label", "Grid.Row", crate::BindingMode::OneWay),
        ]
    );
}

#[test]
fn binding_rules() {
    assert_eq!(
        rejection(r#"<A><B x:Name="b" v="{x:Bind n}" /></A>"#),
        InvalidMarkup::BindingWithoutViewModel
    );
    assert_eq!(
        rejection(r#"<A x:ViewModel="Vm"><B v="{x:Bind n}" /></A>"#),
        InvalidMarkup::BindingWithoutName("B".to_string())
    );
    let bound = |binding: &str| {
        rejection(&format!(
            r#"<A x:ViewModel="Vm"><Border x:Name="b" width="{binding}" /></A>"#
        ))
    };
    assert_eq!(bound("{x:Bind}"), InvalidMarkup::BindingArity);
    assert_eq!(bound("{x:Bind a, b}"), InvalidMarkup::BindingArity);
    assert!(matches!(
        bound("{x:Bind a +}"),
        InvalidMarkup::BindingExpression(_)
    ));
    assert_eq!(
        bound("{x:Bind a, Path=b}"),
        InvalidMarkup::UnknownBindingArgument("Path".to_string())
    );
    assert_eq!(
        bound("{x:Bind a, Mode=Sometimes}"),
        InvalidMarkup::InvalidBindingArgument("Mode".to_string())
    );
    assert_eq!(
        bound("{x:Bind a, Mode={x:Bind b}}"),
        InvalidMarkup::InvalidBindingArgument("Mode".to_string())
    );
    assert_eq!(
        bound("{x:Bind a, BindBack=not-a-name}"),
        InvalidMarkup::InvalidBindingArgument("BindBack".to_string())
    );
    assert_eq!(
        bound("{x:Bind a(), Mode=TwoWay}"),
        InvalidMarkup::TwoWayNeedsTarget
    );
    assert_eq!(
        bound("{x:Bind a, Fallback=1}"),
        InvalidMarkup::FallbackWithoutOptional
    );
    assert_eq!(
        bound("{x:Bind a?.b, Fallback=c?.d}"),
        InvalidMarkup::OptionalFallback
    );
}

#[test]
fn bound_attributes_must_be_ui_properties() {
    assert_eq!(
        rejection(r#"<A x:ViewModel="Vm"><Border x:Name="b" text="{x:Bind t}" /></A>"#),
        InvalidMarkup::UnknownProperty {
            element: "Border".to_string(),
            property: "text".to_string()
        }
    );
}

#[test]
fn other_extensions_are_not_bindings() {
    let doc = document(r#"<A x:ViewModel="Vm"><B v="{StaticResource K}" /></A>"#).unwrap();
    assert!(doc.bindings.is_empty());
}

const EVENTS: &str = r#"<StackPanel x:ViewModel="Vm" x:Name="panel">
    <TextBlock x:Name="title" click="{x:Bind save}" key_pressed="{x:Bind on_key(e)}" text="{x:Bind t}" />
</StackPanel>"#;

#[test]
fn event_attributes_are_handlers_not_bindings() {
    let doc = document(EVENTS).unwrap();
    let handlers: Vec<_> = doc
        .events
        .iter()
        .map(|h| {
            (
                h.field.as_str(),
                h.attribute.as_str(),
                h.method.as_str(),
                h.takes_event,
            )
        })
        .collect();
    assert_eq!(
        handlers,
        [
            ("title", "click", "save", false),
            ("title", "key_pressed", "on_key", true)
        ]
    );
    assert_eq!(doc.events[1].call(), "vm.on_key(event)");
    // `text` is the property; the text event is `text_input`.
    assert_eq!(doc.bindings.len(), 1);
}

#[test]
fn event_handlers_are_checked() {
    let handler = |attr: &str| {
        rejection(&format!(
            r#"<A x:ViewModel="Vm"><TextBlock x:Name="t" click="{attr}" /></A>"#
        ))
    };
    let shape = InvalidMarkup::EventHandlerShape("click".to_string());
    assert_eq!(handler("{x:Bind a.b}"), shape);
    assert_eq!(handler("{x:Bind a(1)}"), shape);
    assert_eq!(handler("{x:Bind a(e, e)}"), shape);
    assert_eq!(
        handler("{x:Bind a, Mode=TwoWay}"),
        InvalidMarkup::EventHandlerArgument("Mode".to_string())
    );
    assert_eq!(
        rejection(r#"<A x:ViewModel="Vm"><TextBlock click="{x:Bind a}" /></A>"#),
        InvalidMarkup::BindingWithoutName("TextBlock".to_string())
    );
}

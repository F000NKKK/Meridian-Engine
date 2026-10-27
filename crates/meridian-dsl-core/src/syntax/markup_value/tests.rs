//! Tests for [`MarkupValue::parse`] and markup extensions.

use super::*;
use crate::MarkupExtension;

fn extension(value: &str) -> MarkupExtension {
    match MarkupValue::parse(value).unwrap() {
        MarkupValue::Extension(extension) => extension,
        MarkupValue::Text(text) => panic!("expected an extension, got text {text:?}"),
    }
}

fn text(value: &str) -> MarkupValue {
    MarkupValue::Text(value.to_string())
}

#[test]
fn plain_and_escaped_values_are_text() {
    assert_eq!(MarkupValue::parse("cube").unwrap(), text("cube"));
    assert_eq!(
        MarkupValue::parse("@mesh:cube").unwrap(),
        text("@mesh:cube")
    );
    assert_eq!(
        MarkupValue::parse("{}{literal}").unwrap(),
        text("{literal}")
    );
    assert_eq!(MarkupValue::parse("").unwrap(), text(""));
}

#[test]
fn bind_with_named_arguments() {
    let bind = extension("{x:Bind name, Mode=TwoWay, BindBack=set_name}");
    assert_eq!(bind.name.to_string(), "x:Bind");
    assert_eq!(bind.positional, vec![text("name")]);
    assert_eq!(bind.argument("Mode"), Some(&text("TwoWay")));
    assert_eq!(bind.argument("BindBack"), Some(&text("set_name")));
}

#[test]
fn commas_and_equals_inside_expressions_stay_positional() {
    let bind = extension("{x:Bind add(1, 2)}");
    assert_eq!(bind.positional, vec![text("add(1, 2)")]);
    let compare = extension("{x:Bind a == b}");
    assert_eq!(compare.positional, vec![text("a == b")]);
    let quoted = extension("{x:Bind join('a,b'), Fallback='x, y'}");
    assert_eq!(quoted.positional, vec![text("join('a,b')")]);
    assert_eq!(quoted.argument("Fallback"), Some(&text("'x, y'")));
}

#[test]
fn nested_extension_as_named_value() {
    let bind = extension("{x:Bind title, Fallback={StaticResource Untitled}}");
    let Some(MarkupValue::Extension(fallback)) = bind.argument("Fallback") else {
        panic!("Fallback should be an extension");
    };
    assert!(fallback.name.is_plain("StaticResource"));
    assert_eq!(fallback.positional, vec![text("Untitled")]);
}

#[test]
fn extension_without_arguments() {
    let bare = extension("{ Empty }");
    assert!(bare.name.is_plain("Empty"));
    assert!(bare.positional.is_empty() && bare.named.is_empty());
}

#[test]
fn malformed_extensions_are_errors() {
    let cases = [
        ("{x:Bind a", MarkupValueError::Unbalanced),
        ("{x:Bind f(a}", MarkupValueError::Unbalanced),
        ("{x:Bind a} tail", MarkupValueError::TrailingContent),
        ("{ }", MarkupValueError::InvalidName),
        ("{y:Bind a}", MarkupValueError::InvalidName),
        ("{x:Bind a,,b}", MarkupValueError::EmptyArgument),
        ("{x:Bind Mode=}", MarkupValueError::EmptyArgument),
        (
            "{x:Bind Mode=OneWay, a}",
            MarkupValueError::PositionalAfterNamed,
        ),
        (
            "{x:Bind a, Mode=OneWay, Mode=TwoWay}",
            MarkupValueError::DuplicateArgument("Mode".to_string()),
        ),
        ("{x:Bind 'open}", MarkupValueError::Unbalanced),
    ];
    for (value, expected) in cases {
        assert_eq!(MarkupValue::parse(value), Err(expected), "{value}");
    }
}

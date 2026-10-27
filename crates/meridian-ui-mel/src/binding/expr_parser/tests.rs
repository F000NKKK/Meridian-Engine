//! Tests for the `{x:Bind}` expression parser and its Rust rendering.

use crate::binding::{Expr, ExprError, ExprErrorKind};

fn rust(text: &str) -> String {
    Expr::parse(text)
        .unwrap_or_else(|error| panic!("{text:?}: {error}"))
        .to_rust()
}

fn error(text: &str) -> ExprError {
    Expr::parse(text).expect_err(text)
}

#[test]
fn members_paths_and_calls() {
    assert_eq!(rust("count"), "vm.count");
    assert_eq!(rust(" state.name "), "vm.state.name");
    assert_eq!(rust("count_text()"), "vm.count_text()");
    assert_eq!(rust("label(id, 2)"), "vm.label(&vm.id, 2)");
    assert_eq!(
        rust("Strings::title(item.id)"),
        "Strings::title(&vm.item.id)"
    );
    assert_eq!(rust("Filter::All"), "Filter::All");
    assert_eq!(rust("items.len()"), "vm.items.len()");
}

#[test]
fn literals_operators_and_optional_access() {
    assert_eq!(rust("'hi'"), r#""hi""#);
    assert_eq!(rust("1.5"), "1.5");
    assert_eq!(rust("true"), "true");
    assert_eq!(rust("!is_open"), "!(vm.is_open)");
    assert_eq!(rust("mode == Filter::All"), "(vm.mode == Filter::All)");
    assert_eq!(rust("(a != 2)"), "(vm.a != 2)");
    assert_eq!(rust("player?.name"), "vm.player.as_ref()?.name");
}

#[test]
fn assignable_and_optional_classification() {
    let parse = |text| Expr::parse(text).unwrap();
    assert!(parse("a.b").is_assignable());
    assert!(!parse("a.b()").is_assignable());
    assert!(!parse("a?.b").is_assignable());
    assert!(!parse("!a").is_assignable());
    assert!(parse("f(a?.b)").is_optional());
    assert!(parse("!a?.b").is_optional());
    assert!(!parse("a.b == 1").is_optional());
}

#[test]
fn malformed_expressions_are_errors_with_offsets() {
    assert_eq!(error("").kind, ExprErrorKind::UnexpectedEnd);
    assert_eq!(error("a +").kind, ExprErrorKind::Unexpected('+'));
    assert_eq!(error("a +").offset, 2);
    assert_eq!(error("'open").kind, ExprErrorKind::UnterminatedText);
    assert_eq!(error("a.").kind, ExprErrorKind::ExpectedName);
    assert_eq!(error("f(a,").kind, ExprErrorKind::UnexpectedEnd);
    assert_eq!(error("(a").kind, ExprErrorKind::UnexpectedEnd);
    assert!(matches!(
        error("1.2.3").kind,
        ExprErrorKind::InvalidNumber(_) | ExprErrorKind::Unexpected(_)
    ));
}

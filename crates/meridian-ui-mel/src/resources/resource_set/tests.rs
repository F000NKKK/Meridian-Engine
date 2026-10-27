//! Tests for [`ResourceSet::parse`].

use super::*;
use crate::ResourceValue;

fn parse(text: &str) -> Result<ResourceSet, ResourceError> {
    ResourceSet::parse(Path::new("Strings.mres"), "Strings", text)
}

fn problem(text: &str) -> ResourceProblem {
    parse(text).unwrap_err().problem
}

#[test]
fn reads_every_value_type() {
    let set = parse(
        r##"{
            // a comment
            mod_name: "Mod List",
            "screen.title": { value: "Yog", comment: "Title" },
            rows: { type: "number", value: 20 },
            ratio: 0.5,
            show: true,
            accent: { type: "color", value: "#FF8800" },
            glass: { type: "color", value: "#80FF8800", },
            at: "@@home",
        }"##,
    )
    .unwrap();
    let values: Vec<_> = set
        .entries
        .iter()
        .map(|e| (e.constant.as_str(), &e.value))
        .collect();
    assert_eq!(
        values,
        [
            ("MOD_NAME", &ResourceValue::Text("Mod List".to_string())),
            ("SCREEN_TITLE", &ResourceValue::Text("Yog".to_string())),
            ("ROWS", &ResourceValue::Integer(20)),
            ("RATIO", &ResourceValue::Float(0.5)),
            ("SHOW", &ResourceValue::Bool(true)),
            ("ACCENT", &ResourceValue::Color(0xFFFF_8800)),
            ("GLASS", &ResourceValue::Color(0x80FF_8800)),
            ("AT", &ResourceValue::Text("@home".to_string())),
        ]
    );
    assert_eq!(
        set.entry("screen.title").unwrap().comment.as_deref(),
        Some("Title")
    );
}

#[test]
fn constants_come_from_snake_case_names() {
    let set = parse(r#"{ ModName: "a", "screen.Title": "b" }"#).unwrap();
    let constants: Vec<_> = set.entries.iter().map(|e| e.constant.as_str()).collect();
    assert_eq!(constants, ["MOD_NAME", "SCREEN_TITLE"]);
}

#[test]
fn mistakes_are_placed_in_the_file() {
    let error = parse("{\n  ok: \"a\",\n  \"bad name\": \"b\",\n}").unwrap_err();
    assert_eq!((error.line, error.column), (3, 3));
    assert_eq!(
        error.problem,
        ResourceProblem::InvalidName("bad name".to_string())
    );
    assert!(
        error.to_string().starts_with("Strings.mres:3:3:"),
        "{error}"
    );
}

#[test]
fn rejects_bad_documents_and_entries() {
    assert!(matches!(problem("{ a: "), ResourceProblem::Syntax(_)));
    assert_eq!(problem("[1]"), ResourceProblem::NotAnObject);
    assert_eq!(
        problem("{ a: { comment: \"x\" } }"),
        ResourceProblem::MissingValue("a".to_string())
    );
    assert_eq!(
        problem("{ a: { value: 1, colour: 2 } }"),
        ResourceProblem::UnknownField {
            entry: "a".to_string(),
            field: "colour".to_string()
        }
    );
    assert_eq!(
        problem(r#"{ a: { type: "sound", value: "x" } }"#),
        ResourceProblem::UnknownType {
            entry: "a".to_string(),
            found: "sound".to_string()
        }
    );
    assert_eq!(
        problem(r#"{ a: { type: "number", value: "x" } }"#),
        ResourceProblem::WrongValue {
            entry: "a".to_string(),
            expected: "a number"
        }
    );
    assert_eq!(
        problem(r##"{ a: { type: "color", value: "#12345" } }"##),
        ResourceProblem::BadColor {
            entry: "a".to_string(),
            found: "#12345".to_string()
        }
    );
    assert_eq!(
        problem("{ a: null }"),
        ResourceProblem::WrongValue {
            entry: "a".to_string(),
            expected: "text"
        }
    );
}

#[test]
fn a_text_cannot_be_a_reference() {
    assert!(matches!(
        problem(r#"{ a: "@file:x.png" }"#),
        ResourceProblem::NotPlainText { .. }
    ));
    assert!(matches!(
        problem(r#"{ a: "@oops" }"#),
        ResourceProblem::NotPlainText { .. }
    ));
}

#[test]
fn names_generating_one_constant_collide() {
    assert_eq!(
        problem(r#"{ "a.b": "x", a_b: "y" }"#),
        ResourceProblem::NameCollision {
            first: "a.b".to_string(),
            second: "a_b".to_string()
        }
    );
}

#[test]
fn set_names_are_type_names() {
    for name in ["", "2d", "my-set", "кот"] {
        let error = ResourceSet::parse(Path::new("x.mres"), name, "{}").unwrap_err();
        assert_eq!(
            error.problem,
            ResourceProblem::InvalidSetName(name.to_string())
        );
    }
}

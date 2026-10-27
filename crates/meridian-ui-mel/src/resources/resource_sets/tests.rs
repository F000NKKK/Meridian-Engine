//! Tests for [`ResourceSets::resolve`].

use std::path::Path;

use super::*;

fn sets() -> ResourceSets {
    let text = r##"{ title: "Yog", at: "@@home", rows: { type: "number", value: 20 },
        accent: { type: "color", value: "#FF8800" }, "screen.name": "Main" }"##;
    let mut sets = ResourceSets::default();
    assert!(sets.insert(ResourceSet::parse(Path::new("Strings.mres"), "Strings", text).unwrap()));
    sets
}

#[test]
fn resources_are_replaced_by_their_values() {
    let sets = sets();
    assert_eq!(sets.resolve("@res:Strings.title").unwrap(), "Yog");
    assert_eq!(sets.resolve("@res:Strings.screen.name").unwrap(), "Main");
    assert_eq!(sets.resolve("@res:Strings.rows").unwrap(), "20");
    assert_eq!(sets.resolve("@res:Strings.accent").unwrap(), "#FFFF8800");
    // A text that starts with '@' stays escaped for the attribute.
    assert_eq!(sets.resolve("@res:Strings.at").unwrap(), "@@home");
}

#[test]
fn other_text_passes_through() {
    let sets = sets();
    assert_eq!(sets.resolve("plain").unwrap(), "plain");
    assert_eq!(sets.resolve("@@literal").unwrap(), "@@literal");
    assert_eq!(sets.resolve("@mesh:hero.mesh").unwrap(), "@mesh:hero.mesh");
}

#[test]
fn unknown_and_malformed_references_are_errors() {
    let sets = sets();
    assert_eq!(
        sets.resolve("@res:Strings.nope"),
        Err(InvalidMarkup::UnknownResource("Strings.nope".to_string()))
    );
    assert_eq!(
        sets.resolve("@res:Other.title"),
        Err(InvalidMarkup::UnknownResource("Other.title".to_string()))
    );
    assert!(matches!(
        sets.resolve("@oops"),
        Err(InvalidMarkup::InvalidLiteral(_))
    ));
    assert!(matches!(
        sets.resolve("@bytes:AAAA"),
        Err(InvalidMarkup::InvalidLiteral(_))
    ));
}

#[test]
fn a_set_name_is_taken_once() {
    let mut sets = sets();
    let again = ResourceSet::parse(Path::new("x/Strings.mres"), "Strings", "{}").unwrap();
    assert!(!sets.insert(again));
}

fn variant(tag: &str, text: &str) -> (Culture, ResourceSet) {
    let culture = Culture::parse(tag).unwrap();
    let path = format!("Strings.{tag}.mres");
    let set = ResourceSet::parse_variant(Path::new(&path), "Strings", culture.clone(), text);
    (culture, set.unwrap())
}

#[test]
fn variants_override_by_specificity_and_fall_back_to_the_default() {
    let mut sets = sets();
    for (tag, text) in [
        ("ru", r#"{ title: "Йог", "screen.name": "Главный" }"#),
        ("ru-ru", r#"{ title: "Йог (РФ)" }"#),
    ] {
        let (culture, set) = variant(tag, text);
        sets.insert_variant(culture, set).unwrap();
    }
    let at = |tag: &str| sets.for_culture(&Culture::parse(tag).unwrap());
    assert_eq!(
        at("ru-ru").resolve("@res:Strings.title").unwrap(),
        "Йог (РФ)"
    );
    assert_eq!(
        at("ru-ru").resolve("@res:Strings.screen.name").unwrap(),
        "Главный"
    );
    assert_eq!(at("ru").resolve("@res:Strings.title").unwrap(), "Йог");
    assert_eq!(at("ru-ua").resolve("@res:Strings.title").unwrap(), "Йог");
    assert_eq!(at("de").resolve("@res:Strings.title").unwrap(), "Yog");
    assert_eq!(at("ru").resolve("@res:Strings.rows").unwrap(), "20");
}

#[test]
fn variants_are_checked_against_the_default() {
    let mut sets = sets();
    let mut insert = |tag: &str, text: &str| {
        let (culture, set) = variant(tag, text);
        sets.insert_variant(culture, set)
    };
    assert_eq!(
        insert("ru", r#"{ extra: "x" }"#),
        Err(ResourceProblem::VariantUnknownEntry {
            set: "Strings".to_string(),
            entry: "extra".to_string()
        })
    );
    assert_eq!(
        insert("ru", r#"{ rows: "many" }"#),
        Err(ResourceProblem::VariantTypeMismatch {
            entry: "rows".to_string()
        })
    );
    assert_eq!(insert("ru", r#"{ title: "Йог" }"#), Ok(()));
    assert_eq!(
        insert("ru", r#"{ title: "again" }"#),
        Err(ResourceProblem::DuplicateSet("Strings.ru".to_string()))
    );
    let orphan = ResourceSet::parse(Path::new("Other.ru.mres"), "Other", "{}").unwrap();
    assert_eq!(
        ResourceSets::default().insert_variant(Culture::parse("ru").unwrap(), orphan),
        Err(ResourceProblem::VariantWithoutDefault("Other".to_string()))
    );
}

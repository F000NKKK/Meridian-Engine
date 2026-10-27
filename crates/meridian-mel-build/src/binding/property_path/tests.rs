//! Tests for [`PropertyPath::resolve`].

use super::*;

fn resolve(tag: &str, attr: &str) -> Result<String, InvalidMarkup> {
    let name = QualifiedName::parse(attr).unwrap();
    PropertyPath::resolve(tag, &name).map(|path| path.to_rust())
}

const P: &str = "::meridian_sdk::ui_core::properties";

#[test]
fn tag_properties_are_found_on_the_tag() {
    assert_eq!(
        resolve("TextBlock", "Text").unwrap(),
        format!("{P}::text_block::TEXT")
    );
    assert_eq!(
        resolve("TextBlock", "text").unwrap(),
        format!("{P}::text_block::TEXT")
    );
    assert_eq!(
        resolve("TextBlock", "font_size").unwrap(),
        format!("{P}::text_block::FONT_SIZE")
    );
}

#[test]
fn every_element_has_the_common_properties() {
    assert_eq!(
        resolve("Border", "width").unwrap(),
        format!("{P}::element::WIDTH")
    );
    assert_eq!(
        resolve("Grid", "HorizontalAlignment").unwrap(),
        format!("{P}::element::HORIZONTAL_ALIGNMENT")
    );
}

#[test]
fn attached_properties_name_their_owner() {
    assert_eq!(
        resolve("TextBlock", "Grid.Row").unwrap(),
        format!("{P}::grid::ROW")
    );
    assert_eq!(
        resolve("Border", "DockPanel.Dock").unwrap(),
        format!("{P}::dock_panel::DOCK")
    );
}

#[test]
fn unknown_properties_are_errors() {
    assert_eq!(
        resolve("TextBlock", "Colour"),
        Err(InvalidMarkup::UnknownProperty {
            element: "TextBlock".to_string(),
            property: "Colour".to_string()
        })
    );
    // `Text` belongs to `TextBlock`, not to `Border`.
    assert!(resolve("Border", "Text").is_err());
    assert!(resolve("Border", "Grid.Nope").is_err());
}

#[test]
fn snake_case_splits_on_capitals() {
    assert_eq!(snake_case("TextBlock"), "text_block");
    assert_eq!(snake_case("Sun2"), "sun2");
    assert_eq!(snake_case("OkButton"), "ok_button");
}

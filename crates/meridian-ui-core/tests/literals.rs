//! Markup literal parsing.

use meridian_ui_core::layout::value::{
    Alignment, Dock, GridLength, Length, Orientation, Visibility,
};
use meridian_ui_core::properties::element::MARGIN;
use meridian_ui_core::properties::grid::COLUMN_DEFINITIONS;
use meridian_ui_core::properties::{self};
use meridian_ui_core::{ElementKind, LiteralError, PropertyType, Thickness, UiError, UiTree};

fn parse<T: PropertyType>(text: &str) -> Result<T, LiteralError> {
    T::parse(text)
}

#[test]
fn lengths_are_auto_or_non_negative_pixels() {
    assert_eq!(parse::<Length>("Auto"), Ok(Length::Auto));
    assert_eq!(parse::<Length>(" 12.5 "), Ok(Length::Px(12.5)));
    assert!(parse::<Length>("-1").is_err());
    assert!(parse::<Length>("auto").is_err());
    assert!(parse::<Length>("NaN").is_err());
}

#[test]
fn grid_lengths_cover_pixels_auto_and_stars() {
    assert_eq!(
        parse::<Vec<GridLength>>("50, Auto, *, 2*"),
        Ok(vec![
            GridLength::Px(50.0),
            GridLength::Auto,
            GridLength::Star(1.0),
            GridLength::Star(2.0),
        ])
    );
    assert_eq!(parse::<Vec<GridLength>>(""), Ok(Vec::new()));
    assert!(parse::<Vec<GridLength>>("0*").is_err());
    assert!(parse::<Vec<GridLength>>("1,,2").is_err());
}

#[test]
fn thickness_takes_one_two_or_four_numbers() {
    assert_eq!(parse::<Thickness>("5"), Ok(Thickness::uniform(5.0)));
    assert_eq!(
        parse::<Thickness>("4,2"),
        Ok(Thickness::new(4.0, 2.0, 4.0, 2.0))
    );
    assert_eq!(
        parse::<Thickness>("1 2 3 4"),
        Ok(Thickness::new(1.0, 2.0, 3.0, 4.0))
    );
    assert!(parse::<Thickness>("1,2,3").is_err());
    assert!(parse::<Thickness>("").is_err());
}

#[test]
fn keywords_are_exact_and_alignment_accepts_either_axis_spelling() {
    assert_eq!(parse::<Alignment>("Left"), Ok(Alignment::Start));
    assert_eq!(parse::<Alignment>("Bottom"), Ok(Alignment::End));
    assert_eq!(
        parse::<Orientation>("Horizontal"),
        Ok(Orientation::Horizontal)
    );
    assert_eq!(parse::<Dock>("Right"), Ok(Dock::Right));
    assert_eq!(parse::<Visibility>("Collapsed"), Ok(Visibility::Collapsed));
    assert!(parse::<Visibility>("collapsed").is_err());
}

#[test]
fn scalars_parse_and_reject_junk() {
    assert_eq!(parse::<bool>("true"), Ok(true));
    assert_eq!(parse::<u32>("7"), Ok(7));
    assert!(parse::<u32>("-1").is_err());
    assert_eq!(parse::<f32>("14"), Ok(14.0));
    assert!(parse::<f32>("inf").is_err());
}

#[test]
fn text_follows_the_at_rule() {
    assert_eq!(parse::<String>("Hello"), Ok("Hello".to_string()));
    assert_eq!(parse::<String>("@@home"), Ok("@home".to_string()));
    assert_eq!(
        parse::<String>("@res:Strings.title"),
        Err(LiteralError::Reference("@res:Strings.title".into()))
    );
}

#[test]
fn set_literal_parses_by_property_and_reports_errors() {
    let mut tree = UiTree::new();
    let node = tree.create(ElementKind::Grid);
    let columns = properties::by_name("Grid.ColumnDefinitions").unwrap().id;
    tree.set_literal(node, columns, "100,*").unwrap();
    assert_eq!(
        tree.get(node, COLUMN_DEFINITIONS).unwrap(),
        vec![GridLength::Px(100.0), GridLength::Star(1.0)]
    );
    tree.set_literal(node, MARGIN.meta().id, "3").unwrap();
    assert_eq!(tree.get(node, MARGIN).unwrap(), Thickness::uniform(3.0));
    assert!(matches!(
        tree.set_literal(node, columns, "wide"),
        Err(UiError::Literal { .. })
    ));
}

#[test]
fn colors_parse_hex_and_names() {
    use meridian_ui_core::Color;
    assert_eq!(parse::<Color>("#fff"), Ok(Color::WHITE));
    assert_eq!(parse::<Color>("#000000"), Ok(Color::BLACK));
    assert_eq!(
        parse::<Color>("#FF000080"),
        Ok(Color::from_rgba8(255, 0, 0, 128))
    );
    assert_eq!(parse::<Color>("Transparent"), Ok(Color::TRANSPARENT));
    assert!(parse::<Color>("#ff").is_err());
    assert!(parse::<Color>("#gggggg").is_err());
    assert!(parse::<Color>("red").is_err());
}

//! [`NamedElement`]: one `x:Name` of a document and the `Name` enum
//! variant generated for it.

use meridian_markup::Span;

use crate::InvalidMarkup;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedElement {
    /// The `x:Name` as written (`left_arm`).
    pub name: String,
    /// The generated variant (`LeftArm`).
    pub variant: String,
    /// The field of the generated `Names` struct (`left_arm`).
    pub field: String,
    /// Where the element starts.
    pub span: Span,
}

impl NamedElement {
    /// Splits `name` on every non-alphanumeric char and joins the parts
    /// in PascalCase; the result must start with a letter.
    pub fn new(name: &str, span: Span) -> Result<Self, InvalidMarkup> {
        let variant: String = name
            .split(|ch: char| !ch.is_ascii_alphanumeric())
            .map(capitalized)
            .collect();
        if !variant.starts_with(|ch: char| ch.is_ascii_alphabetic()) {
            return Err(InvalidMarkup::InvalidName(name.to_string()));
        }
        Ok(Self {
            name: name.to_string(),
            field: crate::snake_case(&variant),
            variant,
            span,
        })
    }
}

fn capitalized(part: &str) -> String {
    let mut chars = part.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_ascii_uppercase().to_string() + chars.as_str()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn variant(name: &str) -> Result<String, InvalidMarkup> {
        let span = Span {
            offset: 0,
            line: 1,
            column: 1,
        };
        NamedElement::new(name, span).map(|named| named.variant)
    }

    #[test]
    fn names_become_pascal_case() {
        assert_eq!(variant("cube").as_deref(), Ok("Cube"));
        assert_eq!(variant("left_arm").as_deref(), Ok("LeftArm"));
        assert_eq!(variant("ok-button").as_deref(), Ok("OkButton"));
        assert_eq!(variant("Sun2").as_deref(), Ok("Sun2"));
        let span = Span {
            offset: 0,
            line: 1,
            column: 1,
        };
        assert_eq!(
            NamedElement::new("ok-button", span).unwrap().field,
            "ok_button"
        );
    }

    #[test]
    fn unusable_names_are_rejected() {
        for name in ["", "2d", "__", "кот"] {
            assert_eq!(
                variant(name),
                Err(InvalidMarkup::InvalidName(name.to_string())),
                "{name}"
            );
        }
    }
}

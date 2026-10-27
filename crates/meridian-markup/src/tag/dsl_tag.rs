//! [`DslTag`]: a type buildable from one element's attributes.

use crate::{Attribute, TagParseError};

/// A type that can be built from one DSL element's attributes.
/// `#[dsl_tag(name = "...")]` generates this impl; nothing prevents
/// writing it by hand for a tag whose parsing doesn't fit the macro's
/// "one attribute per field" model.
pub trait DslTag: Sized + 'static {
    /// The tag name this type binds to, e.g. `"RigidBody"` — matched
    /// exactly against the text of [`crate::Element::name`], case-sensitively.
    const TAG_NAME: &'static str;

    fn from_attrs(attrs: &[Attribute]) -> Result<Self, TagParseError>;
}

/// The text of the plain attribute `name` among `attrs`, for a
/// [`DslTag`] impl building tag `tag`: `Ok(None)` when absent, an error
/// when it is a markup extension (`{x:Bind ...}`), which only the `.mel`
/// generator can resolve (ADR 017), not a tag built straight from markup.
pub fn text_attr<'a>(
    attrs: &'a [Attribute],
    tag: &str,
    name: &str,
) -> Result<Option<&'a str>, TagParseError> {
    let Some(attr) = attrs.iter().find(|attr| attr.name.is_plain(name)) else {
        return Ok(None);
    };
    attr.text().map(Some).ok_or_else(|| TagParseError {
        message: format!(
            "{tag}.{name}: a markup extension needs the .mel generator; this tag takes plain text"
        ),
    })
}

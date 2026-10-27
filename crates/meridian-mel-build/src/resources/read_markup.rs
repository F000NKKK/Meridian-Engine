//! Reading the value of a `<String|Number|Bool|Color x:Key=".." Value=".."/>`
//! resource element of a `.mel` document.

use super::read_entry::{read_color, read_text};
use super::{ResourceProblem, ResourceValue};

/// The tags of resource elements.
pub(crate) const TAGS: [&str; 4] = ["String", "Number", "Bool", "Color"];

/// Reads `text`, the `Value` of the element `<tag x:Key="key">`.
pub(crate) fn read_markup_value(
    key: &str,
    tag: &str,
    text: &str,
) -> Result<ResourceValue, ResourceProblem> {
    let wrong = |expected| ResourceProblem::WrongValue {
        entry: key.to_string(),
        expected,
    };
    match tag {
        "String" => read_text(key, text),
        "Number" => text
            .parse::<i64>()
            .map(ResourceValue::Integer)
            .or_else(|_| text.parse::<f64>().map(ResourceValue::Float))
            .map_err(|_| wrong("a number")),
        "Bool" => text
            .parse::<bool>()
            .map(ResourceValue::Bool)
            .map_err(|_| wrong("true or false")),
        "Color" => read_color(key, text),
        other => Err(ResourceProblem::UnknownType {
            entry: key.to_string(),
            found: other.to_string(),
        }),
    }
}

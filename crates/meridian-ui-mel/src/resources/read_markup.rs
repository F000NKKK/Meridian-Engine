//! Reading the value of a `<String|Number|Bool|Color x:Key=".." Value=".."/>`
//! resource element of a `.mel` document.

use super::read_entry::{read_color, read_text};
use super::resource_tag::ResourceTag;
use super::{ResourceProblem, ResourceValue};

/// Reads `text`, the `Value` of the element `<tag x:Key="key">`.
pub(crate) fn read_markup_value(
    key: &str,
    tag: ResourceTag,
    text: &str,
) -> Result<ResourceValue, ResourceProblem> {
    let wrong = |expected| ResourceProblem::WrongValue {
        entry: key.to_string(),
        expected,
    };
    match tag {
        ResourceTag::String => read_text(key, text),
        ResourceTag::Number => text
            .parse::<i64>()
            .map(ResourceValue::Integer)
            .or_else(|_| text.parse::<f64>().map(ResourceValue::Float))
            .map_err(|_| wrong("a number")),
        ResourceTag::Bool => text
            .parse::<bool>()
            .map(ResourceValue::Bool)
            .map_err(|_| wrong("true or false")),
        ResourceTag::Color => read_color(key, text),
    }
}

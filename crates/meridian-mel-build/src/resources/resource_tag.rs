//! [`ResourceTag`]: the tag of a resource element in a `.meui` document.

/// `<String|Number|Bool|Color x:Key=".." Value=".." />`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResourceTag {
    String,
    Number,
    Bool,
    Color,
}

impl ResourceTag {
    /// The tag spelled `tag`, if it is a resource tag.
    pub(crate) fn parse(tag: &str) -> Option<Self> {
        match tag {
            "String" => Some(Self::String),
            "Number" => Some(Self::Number),
            "Bool" => Some(Self::Bool),
            "Color" => Some(Self::Color),
            _ => None,
        }
    }
}

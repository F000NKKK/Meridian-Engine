//! [`ResourceValue`]: the value of one resource.

#[derive(Debug, Clone, PartialEq)]
pub enum ResourceValue {
    Text(String),
    Integer(i64),
    Float(f64),
    Bool(bool),
    /// `0xAARRGGBB`; `#RRGGBB` is opaque.
    Color(u32),
}

impl ResourceValue {
    /// The value as an attribute of a `.meui` document spells it.
    pub fn literal(&self) -> String {
        match self {
            Self::Text(text) if text.starts_with('@') => format!("@{text}"),
            Self::Text(text) => text.clone(),
            Self::Integer(value) => value.to_string(),
            Self::Float(value) => format!("{value:?}"),
            Self::Bool(value) => value.to_string(),
            Self::Color(argb) => format!("#{argb:08X}"),
        }
    }

    /// The Rust type of the generated constant.
    pub fn rust_type(&self) -> &'static str {
        match self {
            Self::Text(_) => "&'static str",
            Self::Integer(_) => "i64",
            Self::Float(_) => "f64",
            Self::Bool(_) => "bool",
            Self::Color(_) => "u32",
        }
    }

    /// The value as a Rust expression.
    pub fn rust_expr(&self) -> String {
        match self {
            Self::Text(text) => format!("{text:?}"),
            Self::Integer(value) => value.to_string(),
            Self::Float(value) => format!("{value:?}"),
            Self::Bool(value) => value.to_string(),
            Self::Color(argb) => format!("0x{argb:08X}"),
        }
    }
}

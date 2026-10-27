//! [`Argument`]: a named argument an `{x:Bind}` accepts.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Argument {
    Mode,
    BindBack,
    Fallback,
}

impl Argument {
    pub(crate) fn parse(key: &str) -> Option<Self> {
        match key {
            "Mode" => Some(Self::Mode),
            "BindBack" => Some(Self::BindBack),
            "Fallback" => Some(Self::Fallback),
            _ => None,
        }
    }
}

//! [`Mode`]: how a binding flows between the view model and its target.

/// `Mode=` of an `{x:Bind}`; `OneWay` when omitted, as in WPF's `x:Bind`
/// for properties a user does not edit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// Set once, when the view is built.
    OneTime,
    /// View model -> target, whenever the view model changes.
    #[default]
    OneWay,
    /// Both ways: user edits are written back to the view model.
    TwoWay,
}

impl Mode {
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "OneTime" => Some(Self::OneTime),
            "OneWay" => Some(Self::OneWay),
            "TwoWay" => Some(Self::TwoWay),
            _ => None,
        }
    }
}

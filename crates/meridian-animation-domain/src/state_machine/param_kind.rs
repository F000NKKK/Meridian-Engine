//! What a graph parameter holds.

/// What kind of value a parameter holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParamKind {
    /// A number, such as a speed; tested with `Above` and `Below`.
    Float,
    /// A flag that stays as it was set; tested with `Is`.
    Bool,
    /// A one-shot event: set by a trigger, seen by the next update only.
    Trigger,
}

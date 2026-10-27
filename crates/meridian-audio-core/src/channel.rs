//! Output channel names of a speaker layout.

/// A named output channel in a speaker layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
    Left,
    Right,
    Center,
    LowFrequency,
    SurroundLeft,
    SurroundRight,
}

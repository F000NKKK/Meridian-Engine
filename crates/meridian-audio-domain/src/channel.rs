//! Output channel names of a speaker layout.

/// A named output channel in a speaker layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Channel {
    /// Front-left output.
    Left,
    /// Front-right output.
    Right,
    /// Front-center output.
    Center,
    /// Low-frequency effects or subwoofer output.
    LowFrequency,
    /// Left surround output.
    SurroundLeft,
    /// Right surround output.
    SurroundRight,
}

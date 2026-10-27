//! Placement inside the slot a parent hands out.

/// Alignment on one axis. `Start` is left/top, `End` is right/bottom, so
/// one type serves both `HorizontalAlignment` and `VerticalAlignment`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Alignment {
    Start,
    Center,
    End,
    #[default]
    Stretch,
}

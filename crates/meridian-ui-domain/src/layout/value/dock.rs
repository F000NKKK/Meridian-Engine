//! Which edge a `DockPanel` child docks to.

/// The attached `DockPanel.Dock` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Dock {
    #[default]
    Left,
    Top,
    Right,
    Bottom,
}

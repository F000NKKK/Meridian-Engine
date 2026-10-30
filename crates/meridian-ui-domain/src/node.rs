//! Node identity and element kinds.

use meridian_memory::Handle;

/// Generational identity of an element in a [`crate::UiTree`]. Copyable and
/// plain; a removed node's id never aliases a later node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(pub(crate) Handle);

impl NodeId {
    /// Slot index, for tooling that needs a dense key.
    pub fn index(self) -> u32 {
        self.0.index
    }

    /// Generation of the slot when this id was issued.
    pub fn generation(self) -> u32 {
        self.0.generation
    }
}

/// What an element is; selects its layout algorithm and which properties
/// it reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ElementKind {
    /// Single child inset by `Padding`; also the plain container.
    Border,
    /// A bordered slot whose interactive or 3D content is supplied by the host.
    Slot,
    /// Children in a row or column (`Orientation`).
    StackPanel,
    /// Children docked to the edges of the remaining space.
    DockPanel,
    /// Children placed in cells of `Row`/`Column` definitions.
    Grid,
    /// A run of text, sized by a [`crate::TextMeasurer`].
    TextBlock,
    /// One child shown through a window moved by `HorizontalOffset` and
    /// `VerticalOffset`; the child is laid out at its full size.
    ScrollViewer,
}

impl ElementKind {
    /// Every kind, for tooling that lists or round-trips tags.
    pub const ALL: [Self; 7] = [
        Self::Border,
        Self::Slot,
        Self::StackPanel,
        Self::DockPanel,
        Self::Grid,
        Self::TextBlock,
        Self::ScrollViewer,
    ];

    /// The markup tag of this kind (`<StackPanel>`).
    pub const fn tag(self) -> &'static str {
        match self {
            Self::Border => "Border",
            Self::Slot => "Slot",
            Self::StackPanel => "StackPanel",
            Self::DockPanel => "DockPanel",
            Self::Grid => "Grid",
            Self::TextBlock => "TextBlock",
            Self::ScrollViewer => "ScrollViewer",
        }
    }

    /// The kind a markup tag names, if any. Tags are case-sensitive.
    pub fn from_tag(tag: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.tag() == tag)
    }
}

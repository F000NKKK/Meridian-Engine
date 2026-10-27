//! A data-free name for each kind of UI event.

use super::UiEventKind;

/// Which event a [`UiEventKind`] is, without its payload. Markup
/// attributes (`click`, `pointer_pressed`) resolve to a tag at build time,
/// and handlers are matched against the tag of a drained event.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum UiEventTag {
    PointerPressed,
    PointerReleased,
    Click,
    PointerEntered,
    PointerLeft,
    KeyPressed,
    Text,
    FocusGained,
    FocusLost,
}

impl UiEventTag {
    /// Every built-in tag, in declaration order.
    pub const ALL: [Self; 9] = [
        Self::PointerPressed,
        Self::PointerReleased,
        Self::Click,
        Self::PointerEntered,
        Self::PointerLeft,
        Self::KeyPressed,
        Self::Text,
        Self::FocusGained,
        Self::FocusLost,
    ];

    /// The markup attribute name, snake case.
    #[must_use]
    pub const fn attribute(self) -> &'static str {
        match self {
            Self::PointerPressed => "pointer_pressed",
            Self::PointerReleased => "pointer_released",
            Self::Click => "click",
            Self::PointerEntered => "pointer_entered",
            Self::PointerLeft => "pointer_left",
            Self::KeyPressed => "key_pressed",
            Self::Text => "text_input",
            Self::FocusGained => "focus_gained",
            Self::FocusLost => "focus_lost",
        }
    }

    /// The tag a markup attribute names, or `None` for an unknown name.
    #[must_use]
    pub fn from_attribute(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|tag| tag.attribute() == name)
    }

    /// Whether an event of this kind reaches the target's ancestors.
    /// Enter, leave and focus events stay on their target.
    #[must_use]
    pub const fn bubbles(self) -> bool {
        !matches!(
            self,
            Self::PointerEntered | Self::PointerLeft | Self::FocusGained | Self::FocusLost
        )
    }
}

impl UiEventKind {
    /// The payload-free tag of this event.
    #[must_use]
    pub const fn tag(&self) -> UiEventTag {
        match self {
            Self::PointerPressed { .. } => UiEventTag::PointerPressed,
            Self::PointerReleased { .. } => UiEventTag::PointerReleased,
            Self::Click { .. } => UiEventTag::Click,
            Self::PointerEntered => UiEventTag::PointerEntered,
            Self::PointerLeft => UiEventTag::PointerLeft,
            Self::KeyPressed { .. } => UiEventTag::KeyPressed,
            Self::Text(_) => UiEventTag::Text,
            Self::FocusGained => UiEventTag::FocusGained,
            Self::FocusLost => UiEventTag::FocusLost,
        }
    }
}

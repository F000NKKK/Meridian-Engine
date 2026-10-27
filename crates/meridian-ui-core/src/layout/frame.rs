//! The properties every element contributes to its own layout.

use super::value::{Alignment, Length, Visibility};
use crate::properties::element::{
    HEIGHT, HORIZONTAL_ALIGNMENT, MARGIN, VERTICAL_ALIGNMENT, VISIBILITY, WIDTH,
};
use crate::{NodeId, Size, Thickness, UiTree};

/// A snapshot of an element's size, margin and alignment.
#[derive(Debug, Clone, Copy)]
pub(super) struct Frame {
    pub(super) width: Length,
    pub(super) height: Length,
    pub(super) margin: Thickness,
    pub(super) horizontal: Alignment,
    pub(super) vertical: Alignment,
    pub(super) visibility: Visibility,
}

impl Frame {
    pub(super) fn read(tree: &UiTree, node: NodeId) -> Self {
        Self {
            width: tree.get_or_default(node, WIDTH),
            height: tree.get_or_default(node, HEIGHT),
            margin: tree.get_or_default(node, MARGIN),
            horizontal: tree.get_or_default(node, HORIZONTAL_ALIGNMENT),
            vertical: tree.get_or_default(node, VERTICAL_ALIGNMENT),
            visibility: tree.get_or_default(node, VISIBILITY),
        }
    }

    pub(super) fn collapsed(self) -> bool {
        self.visibility == Visibility::Collapsed
    }

    pub(super) fn width_is_explicit(self) -> bool {
        matches!(self.width, Length::Px(_))
    }

    pub(super) fn height_is_explicit(self) -> bool {
        matches!(self.height, Length::Px(_))
    }

    /// The room the element's content may use, given room for the element
    /// including its margin: an explicit size wins over what is on offer.
    pub(super) fn content_available(self, available: Size) -> Size {
        let room = available.deflate(self.margin);
        Size::new(
            explicit_or(self.width, room.width),
            explicit_or(self.height, room.height),
        )
    }

    /// The element's size without margin, once its content wants `content`.
    pub(super) fn own_size(self, content: Size) -> Size {
        Size::new(
            explicit_or(self.width, content.width),
            explicit_or(self.height, content.height),
        )
    }
}

fn explicit_or(length: Length, fallback: f32) -> f32 {
    match length {
        Length::Px(pixels) => pixels,
        Length::Auto => fallback,
    }
}

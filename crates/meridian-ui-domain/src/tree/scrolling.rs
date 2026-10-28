//! Scrolling a `ScrollViewer`.

use super::UiTree;
use crate::layout::{scroll_extent, scroll_offset};
use crate::properties::scroll_viewer::{HORIZONTAL_OFFSET, VERTICAL_OFFSET};
use crate::{ElementKind, NodeId, Point, Size, UiError};

impl UiTree {
    /// The size of a `ScrollViewer`'s content after the last layout; the
    /// viewer's own size for anything else.
    pub fn scroll_extent(&self, node: NodeId) -> Result<Size, UiError> {
        match self.kind(node)? {
            ElementKind::ScrollViewer => Ok(scroll_extent(self, node)),
            _ => Ok(self.layout_rect(node)?.size),
        }
    }

    /// The offsets a `ScrollViewer` is actually at, after clamping.
    pub fn scroll_offset(&self, node: NodeId) -> Result<Point, UiError> {
        self.kind(node)?;
        Ok(scroll_offset(self, node))
    }

    /// Moves a `ScrollViewer`'s content by `delta` pixels (positive scrolls
    /// towards the end), staying inside the content. Returns whether the
    /// offset changed, so callers can tell a viewer at its limit from one
    /// that took the scroll. Takes effect on the next layout.
    pub fn scroll_by(&mut self, node: NodeId, delta: Point) -> Result<bool, UiError> {
        if self.kind(node)? != ElementKind::ScrollViewer {
            return Ok(false);
        }
        let before = self.scroll_offset(node)?;
        let content = self.scroll_extent(node)?;
        let viewport = self.layout_rect(node)?.size;
        let next = Point::new(
            (before.x + delta.x).clamp(0.0, (content.width - viewport.width).max(0.0)),
            (before.y + delta.y).clamp(0.0, (content.height - viewport.height).max(0.0)),
        );
        if next == before {
            return Ok(false);
        }
        self.set(node, HORIZONTAL_OFFSET, next.x)?;
        self.set(node, VERTICAL_OFFSET, next.y)?;
        Ok(true)
    }

    /// The nearest `ScrollViewer` at or above `node`.
    pub(super) fn enclosing_scroll_viewer(&self, node: NodeId) -> Option<NodeId> {
        let mut at = Some(node);
        while let Some(current) = at {
            if self.kind(current) == Ok(ElementKind::ScrollViewer) {
                return Some(current);
            }
            at = self.parent(current).ok().flatten();
        }
        None
    }
}

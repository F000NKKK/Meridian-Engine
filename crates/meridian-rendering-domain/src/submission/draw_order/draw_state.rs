//! How one renderable composites with the frame.

use crate::scene::{BlendMode, Material};

/// The depth and blend behaviour a [`Material`] asks for. The variants are
/// in drawing order: [`draw_order`](super::draw_order) emits them front to
/// back as listed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DrawState {
    /// Depth tested and written, no blending: ordinary solid geometry.
    Opaque,
    /// Depth tested but not written, alpha blended: glass, smoke. Drawn
    /// after every opaque draw, farthest first.
    Transparent,
    /// Ignores depth, no blending: drawn over everything, in submission
    /// order.
    OnTopOpaque,
    /// Ignores depth, alpha blended: a translucent readout over the scene,
    /// drawn farthest first.
    OnTopTransparent,
}

impl DrawState {
    /// The state `material` asks for: `always_on_top` selects the on-top
    /// pair, `blend` selects within the pair.
    #[must_use]
    pub fn of(material: &Material) -> Self {
        match (material.always_on_top, material.blend) {
            (false, BlendMode::Opaque) => Self::Opaque,
            (false, BlendMode::AlphaBlend) => Self::Transparent,
            (true, BlendMode::Opaque) => Self::OnTopOpaque,
            (true, BlendMode::AlphaBlend) => Self::OnTopTransparent,
        }
    }

    /// Whether draws of this state are alpha blended.
    #[must_use]
    pub const fn blends(self) -> bool {
        matches!(self, Self::Transparent | Self::OnTopTransparent)
    }

    /// Whether draws of this state are tested against, and write, depth in
    /// the usual way. On-top states ignore depth, and a transparent draw
    /// tests it without writing so it cannot hide what is behind it.
    #[must_use]
    pub const fn tests_depth(self) -> bool {
        matches!(self, Self::Opaque | Self::Transparent)
    }

    /// Whether draws of this state write depth.
    #[must_use]
    pub const fn writes_depth(self) -> bool {
        matches!(self, Self::Opaque)
    }
}

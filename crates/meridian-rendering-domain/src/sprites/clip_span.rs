//! A clip rectangle applied to a range of layers.

use core::ops::RangeInclusive;

use super::ClipRect;

/// Clips every sprite whose [`Sprite::layer`](crate::scene::Sprite::layer)
/// lies in `layers` to `rect`. Layers are painter's order, so a container
/// that draws its content in one contiguous run of layers (a scroll
/// viewer) clips it with one span. Spans may nest: a sprite under several
/// is clipped to their intersection.
#[derive(Debug, Clone, PartialEq)]
pub struct ClipSpan {
    /// The layers clipped, both ends included.
    pub layers: RangeInclusive<i32>,
    /// Where they show.
    pub rect: ClipRect,
}

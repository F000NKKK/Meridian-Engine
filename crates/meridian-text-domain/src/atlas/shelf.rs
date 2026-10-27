//! One row of the shelf packer.

/// A horizontal strip of the page; bitmaps sit side by side on it.
#[derive(Debug, Clone, Copy)]
pub(super) struct Shelf {
    pub(super) y: u32,
    pub(super) height: u32,
    pub(super) next_x: u32,
}

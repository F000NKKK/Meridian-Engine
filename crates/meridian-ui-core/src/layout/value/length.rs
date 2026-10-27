//! An element's explicit size on one axis.

/// `Width`/`Height`: take the content's size, or a fixed pixel size.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Length {
    #[default]
    Auto,
    Px(f32),
}

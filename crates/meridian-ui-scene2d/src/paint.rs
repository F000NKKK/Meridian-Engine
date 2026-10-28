//! Resources the extraction draws with.

use meridian_rendering_domain::TextureHandle;

/// The GPU-side resources UI drawing needs, supplied by whoever owns the
/// renderer so this crate never creates one.
#[derive(Debug, Clone, Copy)]
pub struct UiPaint {
    /// A 1x1 opaque white texture. Solid fills are sprites of this texture
    /// tinted with the fill color.
    pub fill: TextureHandle,
}

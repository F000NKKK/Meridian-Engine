//! Builds a frame's overlays each tick.

use core::fmt;

use meridian_rendering_domain::Overlay2d;

type BuildOverlays = dyn FnMut() -> Vec<Overlay2d> + Send;

/// The closure a [`RenderStage`](crate::RenderStage) calls each tick for
/// its overlays.
pub(crate) struct OverlayBuilder(Box<BuildOverlays>);

impl OverlayBuilder {
    pub(crate) fn new(build: impl FnMut() -> Vec<Overlay2d> + Send + 'static) -> Self {
        Self(Box::new(build))
    }

    pub(crate) fn build(&mut self) -> Vec<Overlay2d> {
        (self.0)()
    }
}

impl fmt::Debug for OverlayBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OverlayBuilder(..)")
    }
}

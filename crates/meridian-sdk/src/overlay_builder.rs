//! Builds a frame's overlays from the stage context.

use core::fmt;

use meridian_engine_core::StageContext;

use crate::overlay::Overlay2d;

/// The closure a [`RenderStage`](crate::RenderStage) calls each tick for
/// its overlays.
type BuildOverlays = dyn FnMut(&StageContext) -> Vec<Overlay2d> + Send;

pub(crate) struct OverlayBuilder(Box<BuildOverlays>);

impl OverlayBuilder {
    pub(crate) fn new(build: impl FnMut(&StageContext) -> Vec<Overlay2d> + Send + 'static) -> Self {
        Self(Box::new(build))
    }

    pub(crate) fn build(&mut self, ctx: &StageContext) -> Vec<Overlay2d> {
        (self.0)(ctx)
    }
}

impl fmt::Debug for OverlayBuilder {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OverlayBuilder(..)")
    }
}

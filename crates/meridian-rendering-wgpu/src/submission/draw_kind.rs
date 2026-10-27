//! Which pipeline family draws a renderable.

use crate::BindGroup;

/// Which family (colored vs. textured) one [`DrawBuffers`](super::DrawBuffers) entry belongs
/// to, and the per-draw state its main pipeline needs beyond the shared
/// vertex/index buffers.
#[derive(Debug)]
pub(crate) enum DrawKind {
    Colored,
    /// A fresh bind group naming this draw's specific texture — see the
    /// module doc's "no per-texture bind-group caching yet" note.
    Textured {
        bind_group: BindGroup,
    },
}

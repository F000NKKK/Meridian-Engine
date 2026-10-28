//! The buffers one skinned instance reuses from frame to frame.

use std::sync::Arc;

use meridian_rendering_domain::MeshHandle;

use crate::{BindGroup, Buffer};

/// What a skinned instance of `mesh` needs on the GPU: a palette buffer of a
/// fixed size, the posed vertices and the bind group over them. A slot is
/// kept while the same mesh is drawn with a palette of the same size at the
/// same place in the frame's skinned instances.
#[derive(Debug)]
pub(crate) struct SkinSlot {
    pub(crate) mesh: MeshHandle,
    pub(crate) palette: Buffer,
    pub(crate) posed: Arc<Buffer>,
    pub(crate) bind_group: Arc<BindGroup>,
}

impl SkinSlot {
    /// Whether the slot's buffers fit `mesh` posed by a palette of
    /// `palette_bytes` bytes.
    pub(crate) fn fits(&self, mesh: MeshHandle, palette_bytes: usize) -> bool {
        self.mesh == mesh && self.palette.byte_len == palette_bytes
    }
}

//! The buffers one posed instance reuses from frame to frame.

use std::sync::Arc;

use meridian_rendering_domain::MeshHandle;

use crate::{BindGroup, Buffer};

/// What a posed instance of `mesh` needs on the GPU: a palette buffer and a
/// control buffer of fixed sizes, the posed vertices and the bind group over
/// them. A slot is kept while the same mesh is drawn with a palette and
/// morph weights of the same sizes at the same place in the frame's posed
/// instances.
#[derive(Debug)]
pub(crate) struct SkinSlot {
    pub(crate) mesh: MeshHandle,
    pub(crate) palette: Option<Buffer>,
    pub(crate) control: Buffer,
    pub(crate) posed: Arc<Buffer>,
    pub(crate) bind_group: Arc<BindGroup>,
}

/// How big a slot's buffers are.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SlotSizes {
    /// The palette in bytes, none when the mesh is not skinned.
    pub(crate) palette: Option<usize>,
    /// The control block in bytes.
    pub(crate) control: usize,
}

impl SkinSlot {
    /// Whether the slot's buffers fit `mesh` posed with buffers of `sizes`.
    pub(crate) fn fits(&self, mesh: MeshHandle, sizes: SlotSizes) -> bool {
        self.mesh == mesh
            && self.palette.as_ref().map(|palette| palette.byte_len) == sizes.palette
            && self.control.byte_len == sizes.control
    }
}

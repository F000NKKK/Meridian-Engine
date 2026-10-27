//! The GPU buffers and bind group a soft-body kernel keeps between steps.

use meridian_compute_wgpu::{GenerationCache, ScratchSlot};
use meridian_gpu_wgpu::BindGroup;

/// Slots in the order the shader binds them.
pub(super) const SLOT_COUNT: usize = 9;

/// One step's buffers: they are rewritten in place each step and only
/// reallocated when the body outgrows them, and the bind group over them is
/// rebuilt only then (see [`ScratchSlot`], [`GenerationCache`]).
#[derive(Debug, Default)]
pub(super) struct SoftBodyScratch {
    pub(super) slots: [ScratchSlot; SLOT_COUNT],
    pub(super) bind_group: GenerationCache<BindGroup>,
}

impl SoftBodyScratch {
    /// What the bind group was built over: every slot's allocation count.
    pub(super) fn generations(&self) -> [u64; SLOT_COUNT] {
        std::array::from_fn(|index| self.slots[index].generation())
    }
}

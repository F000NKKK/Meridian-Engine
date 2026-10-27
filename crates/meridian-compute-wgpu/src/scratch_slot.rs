//! One GPU buffer that is reused from call to call.

use meridian_gpu_wgpu::{Buffer, BufferUsage};

use crate::GpuComputeDevice;

/// The capacity to allocate when a buffer of `requested` bytes no longer fits
/// in one of `current`: the request, or half again the old capacity if that is
/// more, so a body that grows by a few particles per frame does not
/// reallocate every frame.
fn grown_capacity(current: usize, requested: usize) -> usize {
    requested.max(current + current / 2)
}

/// A GPU buffer that stays allocated between dispatches. [`upload`](Self::upload)
/// rewrites it in place, and allocates a new one only when the data no longer
/// fits (or is meant for another [`BufferUsage`]), so a kernel dispatched every
/// frame stops allocating once its inputs stop growing.
///
/// The buffer may be larger than the data last uploaded: shaders must take
/// sizes from their own parameters, not from the buffer's length, and a
/// read-back must be cut to the length the caller wants.
#[derive(Debug, Default)]
pub struct ScratchSlot {
    buffer: Option<(Buffer, BufferUsage)>,
    generation: u64,
}

impl ScratchSlot {
    /// An empty slot; the first [`upload`](Self::upload) or
    /// [`reserve`](Self::reserve) allocates.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Makes room for `byte_len` bytes meant for `usage` and returns the
    /// buffer. Contents are unspecified when the slot had to reallocate.
    pub fn reserve(
        &mut self,
        gpu: &GpuComputeDevice,
        byte_len: usize,
        usage: BufferUsage,
    ) -> &Buffer {
        let entry = match self.buffer.take() {
            Some((buffer, current)) if current == usage && buffer.byte_len >= byte_len => {
                (buffer, current)
            }
            old => {
                let capacity =
                    grown_capacity(old.map_or(0, |(buffer, _)| buffer.byte_len), byte_len);
                self.generation += 1;
                (gpu.allocate_buffer(capacity, usage), usage)
            }
        };
        &self.buffer.insert(entry).0
    }

    /// [`reserve`](Self::reserve)s room for `bytes` and writes them at offset 0.
    pub fn upload(&mut self, gpu: &GpuComputeDevice, bytes: &[u8], usage: BufferUsage) -> &Buffer {
        let buffer = self.reserve(gpu, bytes.len(), usage);
        gpu.write_buffer(buffer, bytes);
        buffer
    }

    /// The buffer, or `None` before the first allocation.
    #[must_use]
    pub fn buffer(&self) -> Option<&Buffer> {
        self.buffer.as_ref().map(|(buffer, _)| buffer)
    }

    /// How many times this slot has allocated. A bind group built over the
    /// buffer is stale once this changes.
    #[must_use]
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

#[cfg(test)]
mod tests {
    use super::grown_capacity;

    #[test]
    fn a_larger_request_wins_over_the_growth_step() {
        assert_eq!(grown_capacity(100, 1000), 1000);
    }

    #[test]
    fn a_small_overflow_grows_by_half() {
        assert_eq!(grown_capacity(100, 101), 150);
    }

    #[test]
    fn the_first_growth_from_nothing_is_the_request() {
        assert_eq!(grown_capacity(0, 48), 48);
    }
}

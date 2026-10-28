//! Three-dimensional dispatch extents.

/// Device-visible dispatch dimensions for a `ComputeKernel` invocation.
#[derive(Debug, Clone, Copy, Default)]
pub struct DispatchSize {
    pub x: u32,
    pub y: u32,
    pub z: u32,
}

impl DispatchSize {
    /// A 1D dispatch of `count` work items (`y = z = 1`) — what every
    /// current `ComputeKernel` (batch `Motor3` work) uses.
    pub fn linear(count: u32) -> Self {
        Self {
            x: count,
            y: 1,
            z: 1,
        }
    }

    pub fn total(&self) -> usize {
        self.x as usize * self.y as usize * self.z as usize
    }
}

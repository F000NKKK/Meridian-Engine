//! A single-threaded [`ComputeDevice`](crate::ComputeDevice).

use std::any::Any;
use std::ops::Range;

use crate::{ComputeDevice, ComputeSupport};

/// Runs every range on the calling thread, in order: deterministic, with no
/// threads, for tests of code that dispatches through a context.
#[derive(Debug, Clone, Copy, Default)]
pub struct SequentialDevice;

impl ComputeDevice for SequentialDevice {
    fn support(&self) -> ComputeSupport {
        ComputeSupport {
            threads: 1,
            gpu_device_name: None,
        }
    }

    fn dispatch_ranges(&self, count: usize, work: &(dyn Fn(Range<usize>) + Sync)) {
        if count > 0 {
            work(0..count);
        }
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::run_device_contract;

    #[test]
    fn passes_the_device_contract() {
        run_device_contract(|| SequentialDevice);
    }
}

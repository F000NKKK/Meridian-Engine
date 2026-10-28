//! The CPU compute backend.

use std::any::Any;
use std::ops::Range;

use meridian_compute_app::{ComputeDevice, ComputeSupport};
use meridian_platform_winit::CpuCapabilities;

use crate::ComputeCapabilities;

/// The CPU compute backend: implements the [`ComputeDevice`] port with real OS
/// threads.
#[derive(Debug, Clone)]
pub struct CpuComputeDevice {
    capabilities: ComputeCapabilities,
}

impl Default for CpuComputeDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl CpuComputeDevice {
    /// A device sized to this machine's CPU ([`CpuCapabilities::detect`]).
    #[must_use]
    pub fn new() -> Self {
        Self {
            capabilities: ComputeCapabilities {
                cpu: CpuCapabilities::detect(),
                gpu: None,
            },
        }
    }

    /// What this backend reports through the platform's capability shape.
    #[must_use]
    pub fn capabilities(&self) -> ComputeCapabilities {
        self.capabilities.clone()
    }
}

impl ComputeDevice for CpuComputeDevice {
    fn support(&self) -> ComputeSupport {
        ComputeSupport {
            threads: self.capabilities.cpu.threads.max(1),
            gpu_device_name: None,
        }
    }

    /// Splits the range into `threads` contiguous chunks executed on real
    /// worker threads via `std::thread::scope` (borrowed `work`, no `'static`).
    /// Always parallel: the cutoff for small batches is the context's policy.
    fn dispatch_ranges(&self, count: usize, work: &(dyn Fn(Range<usize>) + Sync)) {
        if count == 0 {
            return;
        }
        let threads = self.capabilities.cpu.threads.max(1).min(count);
        let chunk = count.div_ceil(threads);
        let ranges = (0..threads)
            .map(|t| (t * chunk)..((t + 1) * chunk).min(count))
            .filter(|range| range.start < range.end);
        std::thread::scope(|scope| {
            for range in ranges {
                scope.spawn(move || work(range));
            }
        });
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

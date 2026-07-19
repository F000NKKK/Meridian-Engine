//! Low-level compute dispatch abstraction (CPU SIMD backends, GPU compute queues/buffers). Knows nothing about physics, animation or rendering.
//!
//! Only a CPU backend exists so far — no GPU backend, since that needs
//! either an external crate (wgpu/vulkan bindings) or hand-written unsafe
//! FFI neither of which the workspace has taken on yet (see
//! docs/roadmap.md, same call as `platform-core`'s `Window`). The CPU
//! backend is real, not a stub: [`ComputeDevice::dispatch_parallel`] runs
//! work across real OS threads via `std::thread::scope`, safe, no
//! `unsafe`.

use meridian_platform_core::BackendCapabilities;

/// Backend capability flags (CPU SIMD width, GPU compute support).
/// Implements `platform-core`'s [`BackendCapabilities`] — the shape shared
/// with `physics-driver::PhysicsBackend` and future `graphics-driver`/
/// `audio-driver` equivalents; `gpu_compute` is this crate's
/// domain-specific name for the shared "is a GPU backend available" bit.
#[derive(Debug, Clone, Copy, Default)]
pub struct ComputeCapabilities {
    pub gpu_compute: bool,
    /// Worker thread count the CPU backend will fan a dispatch out across.
    pub cpu_threads: usize,
}

impl BackendCapabilities for ComputeCapabilities {
    fn gpu_available(&self) -> bool {
        self.gpu_compute
    }

    fn cpu_threads(&self) -> usize {
        self.cpu_threads
    }
}

/// A device-visible compute buffer. On the CPU backend this is just owned
/// bytes; a GPU backend would instead hold a device-memory handle (not
/// implemented yet).
#[derive(Debug, Clone, Default)]
pub struct ComputeBuffer {
    data: Vec<u8>,
}

impl ComputeBuffer {
    pub fn zeroed(byte_len: usize) -> Self {
        Self {
            data: vec![0u8; byte_len],
        }
    }

    pub fn from_bytes(data: Vec<u8>) -> Self {
        Self { data }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [u8] {
        &mut self.data
    }
}

/// The active compute backend. Only ever the CPU backend right now — see
/// the module doc.
#[derive(Debug, Clone, Copy)]
pub struct ComputeDevice {
    capabilities: ComputeCapabilities,
}

impl Default for ComputeDevice {
    fn default() -> Self {
        Self::new()
    }
}

impl ComputeDevice {
    pub fn new() -> Self {
        Self {
            capabilities: ComputeCapabilities {
                gpu_compute: false,
                cpu_threads: meridian_platform_core::detect_cpu_threads(),
            },
        }
    }

    pub fn capabilities(&self) -> ComputeCapabilities {
        self.capabilities
    }

    pub fn allocate_buffer(&self, byte_len: usize) -> ComputeBuffer {
        ComputeBuffer::zeroed(byte_len)
    }

    /// Runs `work(i)` exactly once for every `i` in `0..count`, splitting
    /// the range into `capabilities().cpu_threads` contiguous chunks
    /// executed on real worker threads via `std::thread::scope` (borrowed
    /// `work`, no `'static`/`Box` needed). Always parallel — callers that
    /// want a small-batch/sequential cutoff apply that policy themselves
    /// (see `meridian-compute-runtime`'s `ComputeContext`, which is the
    /// intended caller); this is mechanism, not policy.
    pub fn dispatch_parallel(&self, count: usize, work: impl Fn(usize) + Sync) {
        if count == 0 {
            return;
        }
        let threads = self.capabilities.cpu_threads.max(1).min(count);
        let chunk = count.div_ceil(threads);
        let work = &work;
        std::thread::scope(|scope| {
            for t in 0..threads {
                let start = t * chunk;
                let end = (start + chunk).min(count);
                if start >= end {
                    continue;
                }
                scope.spawn(move || {
                    for i in start..end {
                        work(i);
                    }
                });
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn capabilities_report_at_least_one_cpu_thread_and_no_gpu() {
        let device = ComputeDevice::new();
        let caps = device.capabilities();
        assert!(caps.cpu_threads >= 1);
        assert!(!caps.gpu_compute);
    }

    #[test]
    fn buffer_zeroed_has_requested_length_and_zero_bytes() {
        let buf = ComputeBuffer::zeroed(16);
        assert_eq!(buf.len(), 16);
        assert!(buf.as_slice().iter().all(|&b| b == 0));
    }

    #[test]
    fn buffer_mut_slice_writes_are_visible() {
        let mut buf = ComputeBuffer::zeroed(4);
        buf.as_mut_slice()[2] = 42;
        assert_eq!(buf.as_slice(), &[0, 0, 42, 0]);
    }

    #[test]
    fn dispatch_parallel_zero_count_calls_work_zero_times() {
        let calls = AtomicUsize::new(0);
        ComputeDevice::new().dispatch_parallel(0, |_| {
            calls.fetch_add(1, Ordering::SeqCst);
        });
        assert_eq!(calls.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn dispatch_parallel_calls_every_index_exactly_once() {
        let n = 10_000;
        let seen: Mutex<Vec<bool>> = Mutex::new(vec![false; n]);
        ComputeDevice::new().dispatch_parallel(n, |i| {
            let mut seen = seen.lock().unwrap();
            assert!(!seen[i], "index {i} visited twice");
            seen[i] = true;
        });
        assert!(
            seen.lock().unwrap().iter().all(|&v| v),
            "every index must have been visited"
        );
    }

    #[test]
    fn dispatch_parallel_matches_sequential_sum() {
        let n = 50_000usize;
        let total = AtomicUsize::new(0);
        ComputeDevice::new().dispatch_parallel(n, |i| {
            total.fetch_add(i, Ordering::Relaxed);
        });
        let expected: usize = (0..n).sum();
        assert_eq!(total.load(Ordering::Relaxed), expected);
    }
}

//! Hardware capability detection and the reporting contract every
//! `*-driver` backend implements (the "devices" view of the platform).

/// Backend capability reporting shared by every `*-driver` crate's own
/// capability type (`compute-driver::ComputeCapabilities`,
/// `physics-driver::PhysicsBackend`, and future `graphics-driver`/
/// `audio-driver` equivalents). CPU and GPU are deliberately separate
/// capability shapes, not one bag of fields with a `gpu: bool` flag: CPU
/// capability is "how many threads" (a number, always present); GPU
/// capability is a completely different shape (device name, VRAM,
/// workgroup limits, ...) that either doesn't exist yet (no backend) or
/// exists with real detected fields (once one does) — `Option<GpuCapabilities>`
/// models that directly instead of a bool that would need a parallel
/// struct bolted on later. When a real GPU backend lands (planned:
/// `wgpu`, not hand-written FFI, unlike `Window`/`DynamicLibrary` — see
/// docs/roadmap.md), it fills in [`GpuCapabilities`]'s fields; no
/// restructuring of this trait or [`CpuCapabilities`] is needed.
pub trait BackendCapabilities {
    fn cpu(&self) -> CpuCapabilities;
    fn gpu(&self) -> Option<GpuCapabilities>;

    fn gpu_available(&self) -> bool {
        self.gpu().is_some()
    }
}

/// CPU-side capability info every `*-driver` backend reports. Each
/// driver's own capability type embeds this (`pub cpu: CpuCapabilities`)
/// instead of redeclaring `threads` itself, and adds its own
/// domain-specific fields (SIMD width, audio latency, ...) alongside it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CpuCapabilities {
    pub threads: usize,
}

impl CpuCapabilities {
    pub fn detect() -> Self {
        Self {
            threads: detect_cpu_threads(),
        }
    }
}

/// GPU-side capability info. `device_name` is real, populated by
/// `graphics-driver::Device` (the first `*-driver` crate with an actual
/// GPU backend — see that crate's module doc for the `wgpu` details);
/// `compute-driver`/`physics-driver`/`audio-driver` still report `gpu:
/// None` since none of them dispatch to a GPU yet. More fields (VRAM, max
/// workgroup size, ...) land here once a backend needs to report them —
/// additive, not a redesign of the `Option`-based shape in
/// [`BackendCapabilities`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GpuCapabilities {
    pub device_name: String,
}

/// Detects the current machine's CPU thread count. Called by
/// [`CpuCapabilities::detect`]; exposed directly too in case a caller
/// needs the thread count without a full `CpuCapabilities`.
pub fn detect_cpu_threads() -> usize {
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn detect_cpu_threads_reports_at_least_one() {
        assert!(detect_cpu_threads() >= 1);
    }
    #[test]
    fn cpu_capabilities_detect_reports_at_least_one_thread() {
        let caps = CpuCapabilities::detect();
        assert!(caps.threads >= 1);
    }
}

//! What a compute backend reports about itself.

/// A compute backend's capabilities, in the application layer's terms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComputeSupport {
    /// Worker threads the CPU side fans out over (at least 1).
    pub threads: usize,
    /// The GPU adapter's name, when a GPU backend is present.
    pub gpu_device_name: Option<String>,
}

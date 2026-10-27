//! The behaviour every [`GpuBackend`](crate::GpuBackend) must have.

use crate::GpuBackend;

/// Asserts the GPU contract against fresh backends from `make`: a non-empty
/// name, and `as_any` yielding the backend itself.
///
/// # Panics
///
/// Panics when `G` breaks the contract.
pub fn run_gpu_contract<G: GpuBackend>(make: impl Fn() -> G) {
    let gpu = make();
    assert!(!gpu.device_name().is_empty(), "a GPU has a name");
    assert!(
        gpu.as_any().downcast_ref::<G>().is_some(),
        "as_any is the backend itself"
    );
}

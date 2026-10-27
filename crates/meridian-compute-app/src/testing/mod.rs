//! Fakes and contract suites for the compute ports (Craftbook 8.4).

mod device_contract;
mod fake_gpu;
mod gpu_contract;
mod sequential_device;

pub use device_contract::run_device_contract;
pub use fake_gpu::FakeGpu;
pub use gpu_contract::run_gpu_contract;
pub use sequential_device::SequentialDevice;

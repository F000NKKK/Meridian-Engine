//! A [`GpuBackend`](crate::GpuBackend) with no GPU behind it.

use std::any::Any;

use crate::GpuBackend;

/// A named GPU backend that computes nothing: lets tests of the context and of
/// hybrid dispatch run where there is no adapter.
#[derive(Debug, Clone)]
pub struct FakeGpu {
    name: String,
}

impl FakeGpu {
    /// A fake adapter called `name`.
    #[must_use]
    pub fn named(name: &str) -> Self {
        Self {
            name: name.to_owned(),
        }
    }
}

impl GpuBackend for FakeGpu {
    fn device_name(&self) -> String {
        self.name.clone()
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::run_gpu_contract;

    #[test]
    fn passes_the_gpu_contract() {
        run_gpu_contract(|| FakeGpu::named("fake"));
    }
}

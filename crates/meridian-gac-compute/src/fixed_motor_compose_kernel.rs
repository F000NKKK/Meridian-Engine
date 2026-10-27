//! Fixed-point motor pair composition, bit-exact on the GPU.

use std::sync::Mutex;

use meridian_compute_runtime::{ComputeContext, ComputeKernel, DispatchSize};
use meridian_gac_core::fixed_ga::FixedMotor3;

/// Mirrors [`MotorComposeKernel`](crate::MotorComposeKernel), built on
/// `gac-core::fixed_ga::FixedMotor3`.
#[derive(Debug)]
pub struct FixedMotorComposeKernel {
    pub pairs: Vec<(FixedMotor3, FixedMotor3)>,
    results: Mutex<Vec<FixedMotor3>>,
}

impl FixedMotorComposeKernel {
    pub fn new(pairs: Vec<(FixedMotor3, FixedMotor3)>) -> Self {
        let results = Mutex::new(vec![FixedMotor3::identity(); pairs.len()]);
        Self { pairs, results }
    }

    pub fn results(&self) -> Vec<FixedMotor3> {
        self.results
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl ComputeKernel for FixedMotorComposeKernel {
    fn dispatch(&self, context: &ComputeContext, size: DispatchSize) {
        let count = size.total().min(self.pairs.len());
        context.parallel_for(count, |i| {
            let (child, parent) = self.pairs[i];
            self.results
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)[i] = child.compose(parent);
        });
    }
}

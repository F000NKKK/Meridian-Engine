//! Fixed-point motor pair composition, bit-exact on the GPU.

use std::sync::Mutex;

use meridian_compute_app::{ComputeContext, ComputeKernel, DispatchSize};
use meridian_gac::FixedMotor3;

/// Mirrors [`MotorComposeKernel`](crate::MotorComposeKernel), built on
/// `gac::FixedMotor3`.
#[derive(Debug)]
pub struct FixedMotorComposeKernel {
    /// The `(child, parent)` pairs to compose, one result per pair.
    pub pairs: Vec<(FixedMotor3, FixedMotor3)>,
    results: Mutex<Vec<FixedMotor3>>,
}

impl FixedMotorComposeKernel {
    /// Creates a kernel over `pairs`; every result starts as the identity
    /// until [`dispatch`](ComputeKernel::dispatch) has run.
    #[must_use]
    pub fn new(pairs: Vec<(FixedMotor3, FixedMotor3)>) -> Self {
        let results = Mutex::new(vec![FixedMotor3::identity(); pairs.len()]);
        Self { pairs, results }
    }

    /// The `pairs[i].0.compose(pairs[i].1)` results from the most recent
    /// dispatch. All-identity if `dispatch` hasn't run yet.
    #[must_use]
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
        context.parallel_ranges(count, |range| {
            let composed: Vec<FixedMotor3> = self.pairs[range.clone()]
                .iter()
                .map(|&(child, parent)| child.compose(parent))
                .collect();
            self.results
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)[range]
                .copy_from_slice(&composed);
        });
    }
}

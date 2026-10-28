//! Composes motor pairs on the GPU.

use std::sync::Mutex;

use meridian_compute_app::{ComputeContext, ComputeKernel, DispatchSize};
use meridian_gac::Motor3;

/// Batch-composes independent `(child, parent)` pairs — `results[i] =
/// pairs[i].0.compose(pairs[i].1)` — as one dispatch. Unlike
/// [`MotorTransformKernel`](crate::MotorTransformKernel), every pair can have a different parent; use
/// this when a batch doesn't share one common parent motor.
#[derive(Debug)]
pub struct MotorComposeKernel {
    pub pairs: Vec<(Motor3, Motor3)>,
    results: Mutex<Vec<Motor3>>,
}

impl MotorComposeKernel {
    pub fn new(pairs: Vec<(Motor3, Motor3)>) -> Self {
        let results = Mutex::new(vec![Motor3::identity(); pairs.len()]);
        Self { pairs, results }
    }

    pub fn results(&self) -> Vec<Motor3> {
        self.results
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl ComputeKernel for MotorComposeKernel {
    fn dispatch(&self, context: &ComputeContext, size: DispatchSize) {
        let count = size.total().min(self.pairs.len());
        context.parallel_ranges(count, |range| {
            let composed: Vec<Motor3> = self.pairs[range.clone()]
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

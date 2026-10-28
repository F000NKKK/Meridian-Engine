//! Fixed-point parent-times-child motor transform, bit-exact on the GPU.

use std::sync::Mutex;

use meridian_compute_app::{ComputeContext, ComputeKernel, DispatchSize};
use meridian_gac::FixedMotor3;

/// Mirrors [`MotorTransformKernel`](crate::MotorTransformKernel), built on
/// `gac::FixedMotor3`. See the module doc for why this
/// (and [`FixedMotorComposeKernel`](crate::FixedMotorComposeKernel)) are CPU-dispatch only.
#[derive(Debug)]
pub struct FixedMotorTransformKernel {
    pub parent: FixedMotor3,
    pub locals: Vec<FixedMotor3>,
    results: Mutex<Vec<FixedMotor3>>,
}

impl FixedMotorTransformKernel {
    pub fn new(parent: FixedMotor3, locals: Vec<FixedMotor3>) -> Self {
        let results = Mutex::new(vec![FixedMotor3::identity(); locals.len()]);
        Self {
            parent,
            locals,
            results,
        }
    }

    pub fn results(&self) -> Vec<FixedMotor3> {
        self.results
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl ComputeKernel for FixedMotorTransformKernel {
    fn dispatch(&self, context: &ComputeContext, size: DispatchSize) {
        let count = size.total().min(self.locals.len());
        context.parallel_ranges(count, |range| {
            let world: Vec<FixedMotor3> = self.locals[range.clone()]
                .iter()
                .map(|local| local.compose(self.parent))
                .collect();
            self.results
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)[range]
                .copy_from_slice(&world);
        });
    }
}

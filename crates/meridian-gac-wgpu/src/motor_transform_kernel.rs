//! Applies a parent motor to many child motors on the GPU.

use std::sync::Mutex;

use meridian_compute_app::{ComputeContext, ComputeKernel, DispatchSize};
use meridian_gac::Motor3;

/// Batch-composes a shared `parent` motor with many `locals` — the
/// `world[i] = locals[i].compose(parent)` step of parent/child transform
/// propagation, done as one dispatch instead of once per entity. This is
/// the entry point `ecs`'s transform propagation, `physics-domain`, and
/// `rendering-domain` use for large batches; single-transform math (`motor *
/// local`) stays a direct `gac` call and never goes through this
/// crate.
#[derive(Debug)]
pub struct MotorTransformKernel {
    pub parent: Motor3,
    pub locals: Vec<Motor3>,
    results: Mutex<Vec<Motor3>>,
}

impl MotorTransformKernel {
    pub fn new(parent: Motor3, locals: Vec<Motor3>) -> Self {
        let results = Mutex::new(vec![Motor3::identity(); locals.len()]);
        Self {
            parent,
            locals,
            results,
        }
    }

    /// The `world[i] = locals[i].compose(parent)` results from the most
    /// recent dispatch. All-identity if `dispatch` hasn't run yet.
    pub fn results(&self) -> Vec<Motor3> {
        self.results
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

impl ComputeKernel for MotorTransformKernel {
    fn dispatch(&self, context: &ComputeContext, size: DispatchSize) {
        let count = size.total().min(self.locals.len());
        context.parallel_ranges(count, |range| {
            let world: Vec<Motor3> = self.locals[range.clone()]
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

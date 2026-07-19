//! Batch execution kernels for `gac-core`'s `Motor3` transforms — the adapter between pure geometric algebra and `compute-runtime`'s dispatch interface. See docs/adr/007-batch-transforms-via-compute.md.

use meridian_gac_core::Motor3;

/// Batch-transforms local-space `Motor3`s into world space as one
/// `compute-runtime` dispatch. This is the entry point `ecs-core`'s
/// transform propagation, `physics-core`, and `graphics-core` use for
/// large batches; single-transform math (`motor * local`) stays a direct
/// `gac-core` call and never goes through this crate. Will implement
/// `compute_runtime::ComputeKernel` once dispatch is implemented.
#[derive(Debug, Clone, Default)]
pub struct MotorTransformKernel {
    pub locals: Vec<Motor3>,
}

/// Batch-composes pairs of `Motor3`s (parent * child) as one dispatch.
#[derive(Debug, Clone, Default)]
pub struct MotorComposeKernel {
    pub pairs: Vec<(Motor3, Motor3)>,
}

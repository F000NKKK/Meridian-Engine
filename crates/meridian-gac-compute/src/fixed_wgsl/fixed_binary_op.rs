//! The binary operations the kernels dispatch.

/// Which [`Fixed`](meridian_numeric_core::Fixed) binary operation a [`FixedArithmeticKernels`](super::FixedArithmeticKernels) pipeline
/// runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixedBinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    /// `atan2(a, b)` (`a` is the y-coordinate) — fits the same `(a, b) ->
    /// one result` dispatch shape as the four arithmetic ops above, so it
    /// reuses [`FixedArithmeticKernels::dispatch`](super::FixedArithmeticKernels::dispatch) rather than needing
    /// its own method the way [`FixedArithmeticKernels::dispatch_sin_cos`](super::FixedArithmeticKernels::dispatch_sin_cos)
    /// does (one input, two outputs, a different buffer shape).
    Atan2,
}

impl FixedBinaryOp {
    pub(crate) fn entry_point(self) -> &'static str {
        match self {
            FixedBinaryOp::Add => "dispatch_add",
            FixedBinaryOp::Sub => "dispatch_sub",
            FixedBinaryOp::Mul => "dispatch_mul",
            FixedBinaryOp::Div => "dispatch_div",
            FixedBinaryOp::Atan2 => "dispatch_atan2",
        }
    }
}

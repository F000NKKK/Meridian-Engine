//! A named reference frame, generic over the flavor.

use crate::flavor::{GaFlavor, MotorLike};

/// A named reference frame: origin + basis, expressed as a motor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Frame<F: GaFlavor> {
    pub motor: F::Motor,
}

impl<F: GaFlavor> Default for Frame<F> {
    fn default() -> Self {
        Self {
            motor: F::Motor::identity(),
        }
    }
}

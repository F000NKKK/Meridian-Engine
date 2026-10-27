//! An opt-in, bit-reproducible `Fixed`-point physics path for lockstep
//! networking and replay (see `meridian_numeric::Fixed` for what that buys
//! and why plain `f32` cannot). It is a parallel API, not a mode switch on the
//! float types. Use [`meridian_gac::FixedMotor3::to_float_lossy`] (and the
//! matching methods on `FixedVec3`/`FixedBivector3`) to hand a
//! deterministically computed pose off to rendering, ECS or audio, which stay in
//! `f32`.

pub use meridian_gac::FixedFlavor;

pub use crate::collision::fixed::*;
pub use crate::dynamics::fixed::*;

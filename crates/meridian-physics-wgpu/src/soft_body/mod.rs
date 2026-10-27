//! GPU soft-body integration for `physics-domain`'s mass-spring model: the
//! per-particle spring adjacency ([`csr`]) both flavors dispatch from, the `f32`
//! kernel in [`float`] and the bit-exact `Fixed` kernel in [`fixed`]. The
//! two WGSL programs are structurally different (native `f32` arithmetic
//! versus emulated Q16.16), which is the GPU-dispatch constraint that
//! keeps this one float/fixed pair from being a generic alias.

pub mod csr;
mod dispatch;
pub mod fixed;
mod fixed_wgsl;
pub mod float;
mod float_wgsl;
mod step_inputs;

#[cfg(test)]
mod tests;

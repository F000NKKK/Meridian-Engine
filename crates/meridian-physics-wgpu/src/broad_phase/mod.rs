//! Batched broad phase: [`BroadPhasePairsKernel`] runs the AABB overlap
//! sweep through `compute-app` for any flavor, and [`float`] holds the
//! WGSL-backed GPU counterpart.

pub mod float;
mod pairs;

#[cfg(test)]
mod tests;

pub use pairs::BroadPhasePairsKernel;

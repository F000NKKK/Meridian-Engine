//! Batched broad phase: [`BroadPhasePairsKernel`] runs the AABB overlap
//! sweep through `compute-runtime` for any flavor, and [`float`] holds the
//! WGSL-backed GPU counterpart.

pub mod float;
mod pairs;

#[cfg(test)]
mod tests;

pub use pairs::BroadPhasePairsKernel;

//! In-memory copies of a world, to roll back to.

mod cloners;
mod error;
#[cfg(test)]
mod tests;
mod world_snapshot;

pub use cloners::Cloners;
pub use error::SnapshotError;
pub use world_snapshot::WorldSnapshot;

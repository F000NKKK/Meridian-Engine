//! The pass graph, its execution order and transient resource planning.

mod allocation_plan;
mod graph;
mod lifetimes;
mod resource_lifetime;

pub use allocation_plan::AllocationPlan;
pub use graph::RenderGraph;
pub use resource_lifetime::ResourceLifetime;

#[cfg(test)]
mod tests;

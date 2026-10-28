//! The domain side of submission: handle-addressed mesh and material registries
//! and the per-view inputs (shadow light, view uniforms). Baking them into GPU
//! draws is the adapter's job (`rendering-wgpu`).
//!
//! Mesh/material identity is handle-based, never a raw index or a driver object
//! ([`MeshRegistry`]/[`MaterialRegistry`] wrap `memory::ResourcePool`,
//! resolved by [`MeshHandle`](crate::MeshHandle)/[`MaterialHandle`](crate::MaterialHandle), ADR 002).
//!
//! Design notes: docs/notes/meridian-rendering-domain.md (`submission`).

mod draw_order;
mod registries;
mod shadows;
#[cfg(test)]
mod tests;
mod view_uniforms;

pub use draw_order::{
    BatchKey, DrawBatch, DrawItem, DrawState, OrderedDraw, order_draws, plan_batches,
};
pub use registries::*;
pub use shadows::*;
pub use view_uniforms::ViewUniforms;

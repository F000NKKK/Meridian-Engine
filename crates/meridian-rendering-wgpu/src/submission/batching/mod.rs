//! What the main pass draws and how it binds it (ADR 046).

mod bind_group_cache;
mod visible_batches;

pub(crate) use bind_group_cache::BindGroupCache;
pub(crate) use visible_batches::VisibleBatches;

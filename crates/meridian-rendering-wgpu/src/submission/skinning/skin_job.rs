//! One skinned instance's compute dispatch.

use std::sync::Arc;

use crate::{BindGroup, Buffer};

/// A dispatch that poses one instance's vertices into `posed`, which the
/// instance's batch then draws instead of the mesh's own vertices.
#[derive(Debug)]
pub(crate) struct SkinJob {
    pub(crate) bind_group: BindGroup,
    pub(crate) workgroups: u32,
    pub(crate) posed: Arc<Buffer>,
}

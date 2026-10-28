//! One instanced draw of a prepared scene.

use core::ops::Range;
use std::sync::Arc;

use meridian_rendering_domain::{DrawState, MeshHandle, TextureHandle};

use super::main_pipelines::PipelineKey;
use crate::BindGroup;

/// Which pipeline family draws a batch.
#[derive(Debug, Clone)]
pub(crate) enum DrawKind {
    /// Flat colour from the instance record.
    Colored,
    /// A bind group naming the batch's albedo texture, made against the
    /// batch's own pipeline, shared with every batch that binds the same
    /// maps (ADR 046).
    Textured { bind_group: Arc<BindGroup> },
}

/// Instances of one mesh that share a pipeline and a bind group, drawn in
/// one call. Built by `bake_batches`
/// before the render pass opens and kept alive until the pass is done.
#[derive(Debug, Clone)]
pub struct PreparedBatch {
    pub(crate) mesh: MeshHandle,
    pub(crate) kind: DrawKind,
    pub(crate) lit: bool,
    pub(crate) state: DrawState,
    pub(crate) casts_shadow: bool,
    /// The batch's emissive map, which bloom samples (ADR 042).
    pub(crate) emissive_map: Option<TextureHandle>,
    /// This batch's records in the scene's instance buffer.
    pub(crate) instances: Range<u32>,
}

impl PreparedBatch {
    /// The main pipeline this batch draws with.
    pub(crate) fn pipeline_key(&self) -> PipelineKey {
        PipelineKey {
            textured: matches!(self.kind, DrawKind::Textured { .. }),
            lit: self.lit,
            state: self.state,
        }
    }

    /// Whether `other` draws with the pipeline and bind group this batch
    /// already has bound, so the draw can leave them as they are.
    pub(crate) fn binds_like(&self, other: &Self) -> bool {
        self.pipeline_key() == other.pipeline_key()
            && match (&self.kind, &other.kind) {
                (DrawKind::Colored, DrawKind::Colored) => true,
                (DrawKind::Textured { bind_group: a }, DrawKind::Textured { bind_group: b }) => {
                    Arc::ptr_eq(a, b)
                }
                _ => false,
            }
    }

    /// How many instances the batch draws.
    #[must_use]
    pub fn instance_count(&self) -> u32 {
        self.instances.end - self.instances.start
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colored(lit: bool, state: DrawState) -> PreparedBatch {
        PreparedBatch {
            mesh: MeshHandle::new(meridian_memory::Handle::default()),
            kind: DrawKind::Colored,
            lit,
            state,
            casts_shadow: true,
            emissive_map: None,
            instances: 0..1,
        }
    }

    #[test]
    fn flat_batches_with_one_pipeline_leave_the_binding_alone() {
        let first = colored(true, DrawState::Opaque);
        assert!(first.binds_like(&colored(true, DrawState::Opaque)));
    }

    #[test]
    fn a_different_pipeline_needs_a_new_binding() {
        let first = colored(true, DrawState::Opaque);
        assert!(!first.binds_like(&colored(false, DrawState::Opaque)));
        assert!(!first.binds_like(&colored(true, DrawState::Transparent)));
    }
}

//! One instanced draw of a prepared scene.

use core::ops::Range;

use meridian_rendering_domain::{DrawState, MeshHandle, TextureHandle};

use super::main_pipelines::PipelineKey;
use crate::BindGroup;

/// Which pipeline family draws a batch.
#[derive(Debug)]
pub(crate) enum DrawKind {
    /// Flat colour from the instance record.
    Colored,
    /// A bind group naming the batch's albedo texture, made against the
    /// batch's own pipeline.
    Textured { bind_group: BindGroup },
}

/// Instances of one mesh that share a pipeline and a bind group, drawn in
/// one call. Built by `bake_batches`
/// before the render pass opens and kept alive until the pass is done.
#[derive(Debug)]
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

    /// How many instances the batch draws.
    #[must_use]
    pub fn instance_count(&self) -> u32 {
        self.instances.end - self.instances.start
    }
}

//! One bind group per pipeline and set of maps, shared by every batch using it.

use std::sync::Arc;

use meridian_rendering_domain::MaterialMaps;

use crate::BindGroup;
use crate::submission::main_pipelines::PipelineKey;

/// The textured bind groups made so far in a frame. Two batches that differ
/// only in their mesh bind the same textures against the same pipeline, so
/// they share the group, and the draw can tell they do (ADR 046).
#[derive(Debug, Default)]
pub(crate) struct BindGroupCache {
    entries: Vec<(PipelineKey, MaterialMaps, Arc<BindGroup>)>,
}

impl BindGroupCache {
    /// The group for `key` and `maps`, made by `make` the first time it is asked for.
    pub(crate) fn group_for(
        &mut self,
        key: PipelineKey,
        maps: MaterialMaps,
        make: impl FnOnce() -> BindGroup,
    ) -> Arc<BindGroup> {
        if let Some((_, _, group)) = self
            .entries
            .iter()
            .find(|(k, m, _)| *k == key && *m == maps)
        {
            return Arc::clone(group);
        }
        let group = Arc::new(make());
        self.entries.push((key, maps, Arc::clone(&group)));
        group
    }
}

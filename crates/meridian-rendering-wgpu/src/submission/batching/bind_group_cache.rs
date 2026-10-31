//! One bind group per pipeline and set of maps, shared by every batch using it.

use std::sync::Arc;

use meridian_rendering_domain::MaterialMaps;

use crate::submission::main_pipelines::PipelineKey;
use crate::{BindGroup, Buffer};

/// The textured bind groups made so far for one prepared view. Two batches
/// that differ only in their mesh share a group, never across cameras.
#[derive(Debug)]
pub(crate) struct BindGroupCache<'a> {
    pub(crate) uniform: &'a Buffer,
    entries: Vec<(PipelineKey, MaterialMaps, Arc<BindGroup>)>,
}

impl<'a> BindGroupCache<'a> {
    pub(crate) fn new(uniform: &'a Buffer) -> Self {
        Self {
            uniform,
            entries: Vec::new(),
        }
    }

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

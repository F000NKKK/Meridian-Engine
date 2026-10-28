//! Which shadow casters each shadow tile draws (ADR 040, ADR 041).

use core::ops::Range;

use meridian_gac::Aabb;
use meridian_rendering_domain::Frustum;

use crate::submission::PreparedBatch;
use crate::submission::instance_data::INSTANCE_STRIDE;
use crate::{Buffer, BufferUsage, Device};

/// A run of one batch's instances, in the cascade's own instance buffer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CasterRun {
    /// Index into the scene's batches.
    pub(crate) batch: usize,
    pub(crate) instances: Range<u32>,
}

/// The shadow casters that reach each cascade's volume: their records
/// copied side by side, with the run of every batch per cascade.
#[derive(Debug, Default, PartialEq, Eq)]
pub(crate) struct CascadeCasters {
    pub(crate) records: Vec<u8>,
    /// One list of runs per tile, in the order of the frusta culled against.
    pub(crate) runs: Vec<Vec<CasterRun>>,
}

impl CascadeCasters {
    /// Keeps, per cascade, the instances of every shadow-casting batch whose
    /// world box meets that cascade's volume. An instance without bounds is
    /// kept: an unknown box must not make a shadow vanish.
    pub(crate) fn cull(
        batches: &[PreparedBatch],
        records: &[u8],
        bounds: &[Option<Aabb>],
        frusta: &[Frustum],
    ) -> Self {
        let mut casters = Self::default();
        for frustum in frusta {
            let runs = casters.copy_tile(batches, records, bounds, frustum);
            casters.runs.push(runs);
        }
        casters
    }

    /// Appends one tile's surviving records and returns their runs.
    fn copy_tile(
        &mut self,
        batches: &[PreparedBatch],
        records: &[u8],
        bounds: &[Option<Aabb>],
        frustum: &Frustum,
    ) -> Vec<CasterRun> {
        let mut runs = Vec::new();
        for (index, batch) in batches.iter().enumerate().filter(|(_, b)| b.casts_shadow) {
            let first = self.count();
            for at in batch.instances.clone().map(|instance| instance as usize) {
                if bounds[at].is_some_and(|aabb| !frustum.intersects(&aabb)) {
                    continue;
                }
                let start = at * INSTANCE_STRIDE;
                self.records
                    .extend_from_slice(&records[start..start + INSTANCE_STRIDE]);
            }
            if self.count() > first {
                runs.push(CasterRun {
                    batch: index,
                    instances: first..self.count(),
                });
            }
        }
        runs
    }

    /// How many records have been copied so far.
    fn count(&self) -> u32 {
        u32::try_from(self.records.len() / INSTANCE_STRIDE).unwrap_or(u32::MAX)
    }

    /// The records on the GPU, `None` when no cascade draws anything.
    pub(crate) fn upload(&self, device: &Device) -> Option<Buffer> {
        if self.records.is_empty() {
            return None;
        }
        let buffer = device.create_buffer(self.records.len(), BufferUsage::Vertex);
        device.write_buffer(&buffer, &self.records);
        Some(buffer)
    }
}

#[cfg(test)]
mod tests {
    use meridian_gac::{Motor3, Projection, Vec3};
    use meridian_rendering_domain::{
        Camera, DrawState, MeshHandle, SHADOW_CASCADES, ShadowCascades,
    };

    use super::*;
    use crate::submission::DrawKind;

    fn batch(casts_shadow: bool, instances: Range<u32>) -> PreparedBatch {
        PreparedBatch {
            mesh: MeshHandle::new(meridian_memory::Handle::default()),
            kind: DrawKind::Colored,
            lit: true,
            state: DrawState::Opaque,
            casts_shadow,
            emissive_map: None,
            instances,
            vertices: None,
        }
    }

    fn unit_box(center: Vec3) -> Aabb {
        let half = Vec3::new(0.25, 0.25, 0.25);
        Aabb {
            min: center - half,
            max: center + half,
        }
    }

    /// Records whose every byte is the instance's index.
    fn records(count: u8) -> Vec<u8> {
        (0..count)
            .flat_map(|index| [index; INSTANCE_STRIDE])
            .collect()
    }

    fn frusta() -> [Frustum; SHADOW_CASCADES] {
        let camera = Camera {
            frame: Motor3::identity(),
            projection: Projection::perspective(1.0, 1.0, 0.1, 500.0),
        };
        ShadowCascades::fit(&camera, Vec3::new(0.3, -1.0, 0.2).normalize(), 1024).frusta()
    }

    #[test]
    fn a_far_off_caster_is_left_out_of_every_cascade() {
        let bounds = [
            Some(unit_box(Vec3::new(2.0, 0.0, 0.0))),
            Some(unit_box(Vec3::new(0.0, 0.0, 400.0))),
        ];
        let casters = CascadeCasters::cull(&[batch(true, 0..2)], &records(2), &bounds, &frusta());
        assert_eq!(
            casters.runs[0],
            [CasterRun {
                batch: 0,
                instances: 0..1
            }]
        );
        let drawn = casters.runs.iter().flatten();
        assert!(drawn.clone().all(|run| run.instances.len() == 1));
        assert!(
            casters
                .records
                .chunks(INSTANCE_STRIDE)
                .all(|record| record[0] == 0)
        );
    }

    #[test]
    fn a_batch_that_casts_no_shadow_is_never_drawn() {
        let bounds = [Some(unit_box(Vec3::new(2.0, 0.0, 0.0)))];
        let casters = CascadeCasters::cull(&[batch(false, 0..1)], &records(1), &bounds, &frusta());
        assert!(casters.runs.iter().all(Vec::is_empty));
        assert_eq!(casters.records, Vec::new());
    }

    #[test]
    fn an_instance_without_bounds_is_kept() {
        let casters = CascadeCasters::cull(&[batch(true, 0..1)], &records(1), &[None], &frusta());
        assert!(casters.runs.iter().all(|runs| runs.len() == 1));
    }

    #[test]
    fn runs_index_the_copied_records_of_each_cascade_in_turn() {
        let bounds = [
            Some(unit_box(Vec3::new(2.0, 0.0, 0.0))),
            Some(unit_box(Vec3::new(3.0, 0.0, 0.0))),
        ];
        let batches = [batch(true, 0..1), batch(true, 1..2)];
        let casters = CascadeCasters::cull(&batches, &records(2), &bounds, &frusta());
        assert_eq!(casters.runs[0][1].instances, 1..2);
        let next = casters.runs[1][0].instances.clone();
        assert_eq!(next.start, 2, "cascade 1 starts after cascade 0's records");
    }
}

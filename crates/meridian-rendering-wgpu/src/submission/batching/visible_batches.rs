//! The batches the camera can see (ADR 046).

use meridian_gac::Aabb;
use meridian_rendering_domain::Frustum;

use crate::submission::PreparedBatch;
use crate::submission::instance_data::INSTANCE_STRIDE;
use crate::{Buffer, BufferUsage, Device};

/// The instances of a scene's batches that meet the camera's frustum, their
/// records copied side by side and each batch's range rewritten to its
/// copy. A batch with nothing visible is dropped.
#[derive(Debug, Default)]
pub(crate) struct VisibleBatches {
    pub(crate) batches: Vec<PreparedBatch>,
    pub(crate) records: Vec<u8>,
}

impl VisibleBatches {
    /// Keeps, in drawing order, the instances of `batches` whose world box
    /// meets `frustum`. An instance without bounds is kept: an unknown box
    /// must not make an object vanish. `records` and `bounds` are indexed by
    /// the batches' original ranges.
    pub(crate) fn cull(
        batches: &[PreparedBatch],
        records: &[u8],
        bounds: &[Option<Aabb>],
        frustum: &Frustum,
    ) -> Self {
        let mut visible = Self::default();
        for batch in batches {
            let first = visible.count();
            for at in batch.instances.clone().map(|instance| instance as usize) {
                if bounds[at].is_some_and(|aabb| !frustum.intersects(&aabb)) {
                    continue;
                }
                let start = at * INSTANCE_STRIDE;
                visible
                    .records
                    .extend_from_slice(&records[start..start + INSTANCE_STRIDE]);
            }
            if visible.count() > first {
                visible.batches.push(PreparedBatch {
                    instances: first..visible.count(),
                    ..batch.clone()
                });
            }
        }
        visible
    }

    /// How many records have been kept so far.
    fn count(&self) -> u32 {
        u32::try_from(self.records.len() / INSTANCE_STRIDE).unwrap_or(u32::MAX)
    }

    /// The records on the GPU, `None` when nothing is visible.
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
    use meridian_rendering_domain::{Camera, DrawState, MeshHandle};

    use super::*;
    use crate::submission::DrawKind;

    fn batch(instances: core::ops::Range<u32>) -> PreparedBatch {
        PreparedBatch {
            mesh: MeshHandle::new(meridian_memory::Handle::default()),
            kind: DrawKind::Colored,
            lit: true,
            state: DrawState::Opaque,
            casts_shadow: true,
            emissive_map: None,
            instances,
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

    /// A camera at the origin looking down +X.
    fn frustum() -> Frustum {
        let camera = Camera {
            frame: Motor3::identity(),
            projection: Projection::perspective(1.0, 1.0, 0.1, 50.0),
        };
        Frustum::from_view_projection(camera.view_projection_matrix())
    }

    #[test]
    fn instances_outside_the_view_are_dropped_and_the_rest_packed() {
        let bounds = [
            Some(unit_box(Vec3::new(5.0, 0.0, 0.0))),
            Some(unit_box(Vec3::new(-5.0, 0.0, 0.0))),
            Some(unit_box(Vec3::new(8.0, 0.0, 0.0))),
        ];
        let visible = VisibleBatches::cull(&[batch(0..3)], &records(3), &bounds, &frustum());
        assert_eq!(visible.batches.len(), 1);
        assert_eq!(visible.batches[0].instances, 0..2);
        let kept: Vec<u8> = visible
            .records
            .chunks(INSTANCE_STRIDE)
            .map(|record| record[0])
            .collect();
        assert_eq!(kept, [0, 2], "the one behind the camera is gone");
    }

    #[test]
    fn a_batch_with_nothing_visible_is_dropped_and_ranges_follow_the_copy() {
        let bounds = [
            Some(unit_box(Vec3::new(-5.0, 0.0, 0.0))),
            Some(unit_box(Vec3::new(5.0, 0.0, 0.0))),
            Some(unit_box(Vec3::new(6.0, 0.0, 0.0))),
        ];
        let batches = [batch(0..1), batch(1..3)];
        let visible = VisibleBatches::cull(&batches, &records(3), &bounds, &frustum());
        assert_eq!(visible.batches.len(), 1);
        assert_eq!(visible.batches[0].instances, 0..2);
    }

    #[test]
    fn an_instance_without_bounds_is_kept() {
        let visible = VisibleBatches::cull(&[batch(0..1)], &records(1), &[None], &frustum());
        assert_eq!(visible.batches[0].instances, 0..1);
    }
}

//! GPU time of every render pass of a frame, read back from timestamp queries.

use std::sync::{Mutex, mpsc};

use crate::Device;

/// Passes one frame may record; a frame past it simply times the first ones.
const MAX_PASSES: u32 = 64;
const TIMESTAMP_BYTES: u64 = 8;

/// How long one named pass took on the GPU, summed over every pass of the
/// frame that carries the same name.
#[derive(Debug, Clone, PartialEq)]
pub struct PassTiming {
    /// The name the pass was recorded under.
    pub label: &'static str,
    /// GPU time, in milliseconds.
    pub millis: f64,
}

/// Timestamp queries around render passes. Exists only on a device that
/// offers them ([`PassProfiler::new`] returns `None` otherwise), is attached
/// to a [`CommandBuffer`](crate::CommandBuffer) per frame, and is read back
/// with [`PassProfiler::collect`] once that frame is submitted.
#[derive(Debug)]
pub struct PassProfiler {
    queries: wgpu::QuerySet,
    resolve: wgpu::Buffer,
    readback: wgpu::Buffer,
    labels: Mutex<Vec<&'static str>>,
    nanos_per_tick: f64,
}

impl PassProfiler {
    /// A profiler for `device`, or `None` when it has no timestamp queries.
    #[must_use]
    pub fn new(device: &Device) -> Option<Self> {
        if !device.0.supports_timestamps() {
            return None;
        }
        let raw = device.0.wgpu_device();
        let count = MAX_PASSES * 2;
        let size = u64::from(count) * TIMESTAMP_BYTES;
        Some(Self {
            queries: raw.create_query_set(&wgpu::QuerySetDescriptor {
                label: Some("pass timestamps"),
                ty: wgpu::QueryType::Timestamp,
                count,
            }),
            resolve: raw.create_buffer(&wgpu::BufferDescriptor {
                label: Some("pass timestamps resolve"),
                size,
                usage: wgpu::BufferUsages::QUERY_RESOLVE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            readback: raw.create_buffer(&wgpu::BufferDescriptor {
                label: Some("pass timestamps readback"),
                size,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }),
            labels: Mutex::new(Vec::new()),
            nanos_per_tick: f64::from(device.0.wgpu_queue().get_timestamp_period()),
        })
    }

    /// The timestamp writes of the next pass, named `label`; `None` once the
    /// frame has used up its queries.
    pub(crate) fn claim(&self, label: &'static str) -> Option<wgpu::RenderPassTimestampWrites<'_>> {
        let mut labels = self.labels.lock().ok()?;
        let slot = u32::try_from(labels.len()).ok()?;
        if slot >= MAX_PASSES {
            return None;
        }
        labels.push(label);
        Some(wgpu::RenderPassTimestampWrites {
            query_set: &self.queries,
            beginning_of_pass_write_index: Some(slot * 2),
            end_of_pass_write_index: Some(slot * 2 + 1),
        })
    }

    /// Records the copy of the frame's timestamps to the readback buffer.
    pub(crate) fn resolve(&self, encoder: &mut wgpu::CommandEncoder) {
        let used = self.labels.lock().map_or(0, |labels| labels.len());
        let count = u32::try_from(used).unwrap_or(0) * 2;
        if count == 0 {
            return;
        }
        let bytes = u64::from(count) * TIMESTAMP_BYTES;
        encoder.resolve_query_set(&self.queries, 0..count, &self.resolve, 0);
        encoder.copy_buffer_to_buffer(&self.resolve, 0, &self.readback, 0, bytes);
    }

    /// Waits for the submitted frame and returns its passes' GPU times,
    /// in order of first appearance, then starts the next frame.
    #[must_use]
    pub fn collect(&self, device: &Device) -> Vec<PassTiming> {
        let labels = self
            .labels
            .lock()
            .map(|mut labels| std::mem::take(&mut *labels))
            .unwrap_or_default();
        if labels.is_empty() {
            return Vec::new();
        }
        let bytes = labels.len() as u64 * 2 * TIMESTAMP_BYTES;
        let slice = self.readback.slice(..bytes);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            drop(sender.send(result));
        });
        if device
            .0
            .wgpu_device()
            .poll(wgpu::PollType::wait_indefinitely())
            .is_err()
        {
            return Vec::new();
        }
        if !matches!(receiver.recv(), Ok(Ok(()))) {
            return Vec::new();
        }
        let Ok(view) = slice.get_mapped_range() else {
            return Vec::new();
        };
        let ticks: Vec<u64> = view
            .chunks_exact(8)
            .map(|chunk| u64::from_le_bytes(chunk.try_into().unwrap_or([0; 8])))
            .collect();
        drop(view);
        self.readback.unmap();
        self.sum_by_label(&labels, &ticks)
    }

    #[expect(
        clippy::cast_precision_loss,
        reason = "tick deltas of one frame are far below 2^52"
    )]
    fn sum_by_label(&self, labels: &[&'static str], ticks: &[u64]) -> Vec<PassTiming> {
        let mut timings: Vec<PassTiming> = Vec::new();
        for (label, pair) in labels.iter().zip(ticks.chunks_exact(2)) {
            let millis = pair[1].saturating_sub(pair[0]) as f64 * self.nanos_per_tick / 1.0e6;
            match timings.iter_mut().find(|timing| timing.label == *label) {
                Some(timing) => timing.millis += millis,
                None => timings.push(PassTiming { label, millis }),
            }
        }
        timings
    }
}

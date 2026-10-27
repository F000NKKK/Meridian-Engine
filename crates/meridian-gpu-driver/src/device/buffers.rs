//! Buffer creation, upload and readback on the device.

use crate::Buffer;
use crate::BufferUsage;
use crate::Device;
use std::sync::mpsc;

impl Device {
    /// Allocates a GPU-visible buffer of `byte_len` bytes. Always usable
    /// as a copy source/destination in addition to `usage`'s role — a
    /// generous default rather than fine-grained per-buffer usage-flag
    /// control, which is future work once a concrete need for tighter
    /// flags shows up.
    pub fn create_buffer(&self, byte_len: usize, usage: BufferUsage) -> Buffer {
        let raw = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: byte_len as u64,
            usage: usage.to_wgpu() | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        Buffer { raw, byte_len }
    }

    /// Uploads `data` to `buffer` starting at byte offset 0.
    pub fn write_buffer(&self, buffer: &Buffer, data: &[u8]) {
        self.queue.write_buffer(&buffer.raw, 0, data);
    }

    /// Reads `buffer`'s entire contents back to the CPU. A real `async
    /// fn` — see the module doc. Copies into a `MAP_READ`-capable staging
    /// buffer first (most `buffer` usages, e.g. `STORAGE`, aren't
    /// directly mappable on discrete GPUs) — the standard `wgpu` readback
    /// pattern. `wgpu` has no reactor of its own to drive `map_async`'s
    /// completion callback — something has to call `wgpu::Device::poll`,
    /// which itself blocks the calling thread until the GPU catches up —
    /// so that poll runs inside [`tokio::task::spawn_blocking`] rather
    /// than on this `async fn`'s own task.
    pub async fn read_buffer(&self, buffer: &Buffer) -> Vec<u8> {
        let staging = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("meridian-gpu-driver readback staging buffer"),
            size: buffer.byte_len as u64,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_buffer_to_buffer(&buffer.raw, 0, &staging, 0, buffer.byte_len as u64);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging.slice(..);
        let (tx, rx) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            // A closed channel means the reading future was dropped before
            // the map finished; the staging buffer is freed with it.
            if tx.send(result).is_err() {
                meridian_foundation::log_debug!("read_buffer: map finished after the reader left");
            }
        });

        let device = self.device.clone();
        tokio::task::spawn_blocking(move || device.poll(wgpu::PollType::wait_indefinitely()))
            .await
            .expect("device poll task panicked")
            .expect("device poll failed");
        rx.recv()
            .expect("map_async callback dropped without a response")
            .expect("failed to map staging buffer for readback");

        let data = slice
            .get_mapped_range()
            .expect("staging buffer was mapped but its range isn't readable")
            .to_vec();
        staging.unmap();
        data
    }
}

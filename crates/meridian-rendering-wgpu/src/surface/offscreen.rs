//! A surface that renders into a texture instead of a window, and reads it back.

use std::sync::mpsc;

use super::{Surface, SurfaceTarget};
use crate::Device;

/// 8-bit sRGB: the tone-mapped output reads back as ordinary image bytes.
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;
const BYTES_PER_PIXEL: u32 = 4;

pub(super) fn offscreen_texture(device: &Device, width: u32, height: u32) -> wgpu::Texture {
    device
        .0
        .wgpu_device()
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("offscreen surface"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        })
}

impl Surface {
    /// A surface with no window: frames render into a `width` x `height`
    /// texture that [`read_pixels`](Self::read_pixels) returns. Works with a
    /// headless [`Device::new`], so a renderer built on it needs no display.
    #[must_use]
    pub fn offscreen(device: &Device, width: u32, height: u32) -> Self {
        let (width, height) = (width.max(1), height.max(1));
        Self {
            target: SurfaceTarget::Offscreen(offscreen_texture(device, width, height)),
            config: wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format: FORMAT,
                color_space: wgpu::SurfaceColorSpace::Auto,
                width,
                height,
                present_mode: wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: 2,
                alpha_mode: wgpu::CompositeAlphaMode::Opaque,
                view_formats: Vec::new(),
            },
        }
    }

    /// The last frame as tightly packed RGBA8 rows, top first; `None` for a
    /// window surface or when the readback fails. Waits for the GPU.
    #[must_use]
    pub fn read_pixels(&self, device: &Device) -> Option<Vec<u8>> {
        let SurfaceTarget::Offscreen(texture) = &self.target else {
            return None;
        };
        let (width, height) = (self.config.width, self.config.height);
        let row = width * BYTES_PER_PIXEL;
        let padded = row.next_multiple_of(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT);
        let raw = device.0.wgpu_device();
        let buffer = raw.create_buffer(&wgpu::BufferDescriptor {
            label: Some("offscreen readback"),
            size: u64::from(padded) * u64::from(height),
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let mut encoder =
            raw.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        encoder.copy_texture_to_buffer(
            texture.as_image_copy(),
            wgpu::TexelCopyBufferInfo {
                buffer: &buffer,
                layout: wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(padded),
                    rows_per_image: Some(height),
                },
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        device.0.wgpu_queue().submit(Some(encoder.finish()));
        let slice = buffer.slice(..);
        let (sender, receiver) = mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |result| {
            drop(sender.send(result));
        });
        raw.poll(wgpu::PollType::wait_indefinitely()).ok()?;
        receiver.recv().ok()?.ok()?;
        let view = slice.get_mapped_range().ok()?;
        let mut pixels = Vec::with_capacity((row * height) as usize);
        for line in view.chunks_exact(padded as usize) {
            pixels.extend_from_slice(&line[..row as usize]);
        }
        drop(view);
        buffer.unmap();
        Some(pixels)
    }
}

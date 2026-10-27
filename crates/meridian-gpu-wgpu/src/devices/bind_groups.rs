//! Bind group creation on the device.

use crate::{BindGroup, Buffer, Device, SampledTexture, Sampler, Texture};

impl Device {
    /// Builds a bind group binding `buffer` at `@group(0) @binding(0)` of
    /// `pipeline`'s auto-derived layout (see
    /// [`Device::create_compute_pipeline`](crate::Device::create_compute_pipeline)'s doc comment). Shared by the
    /// compute-dispatch path here and by `graphics-driver`'s uniform bind
    /// group for its render pipelines — same single-binding shape either
    /// way.
    pub fn create_single_buffer_bind_group(
        &self,
        pipeline_layout: &wgpu::BindGroupLayout,
        buffer: &Buffer,
    ) -> BindGroup {
        self.create_bind_group(pipeline_layout, &[buffer])
    }

    /// Builds a bind group binding `buffers[i]` at `@group(0)
    /// @binding(i)` of `pipeline_layout`, for pipelines that need more
    /// than one bound resource (e.g. a compute kernel reading from one
    /// storage buffer and writing to another) —
    /// [`Device::create_single_buffer_bind_group`](crate::Device::create_single_buffer_bind_group)'s general form.
    pub fn create_bind_group(
        &self,
        pipeline_layout: &wgpu::BindGroupLayout,
        buffers: &[&Buffer],
    ) -> BindGroup {
        let entries: Vec<wgpu::BindGroupEntry> = buffers
            .iter()
            .enumerate()
            .map(|(i, buffer)| wgpu::BindGroupEntry {
                binding: i as u32,
                resource: buffer.raw.as_entire_binding(),
            })
            .collect();
        let raw = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: pipeline_layout,
            entries: &entries,
        });
        BindGroup { raw }
    }

    /// Builds a bind group binding `buffer` at `@group(0) @binding(0)`,
    /// `texture`'s default view at `@binding(1)` and `sampler` at
    /// `@binding(2)` of `pipeline_layout` — the shape a shader that
    /// samples one texture, alongside one uniform buffer, needs. See
    /// [`create_bind_group`](Self::create_bind_group) for the
    /// buffers-only general form this specializes.
    pub fn create_texture_bind_group(
        &self,
        pipeline_layout: &wgpu::BindGroupLayout,
        buffer: &Buffer,
        texture: &Texture,
        sampler: &Sampler,
    ) -> BindGroup {
        let raw = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: pipeline_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.raw.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler.raw),
                },
            ],
        });
        BindGroup { raw }
    }

    /// [`create_texture_bind_group`](Self::create_texture_bind_group)
    /// plus a shadow map at `@binding(3)` and its comparison sampler at
    /// `@binding(4)` — the shape a textured, shadow-receiving lit shader
    /// needs (uniform, albedo texture, albedo sampler, shadow map,
    /// shadow sampler, in that binding order).
    pub fn create_textured_shadow_bind_group(
        &self,
        pipeline_layout: &wgpu::BindGroupLayout,
        buffer: &Buffer,
        albedo: SampledTexture<'_>,
        shadow: SampledTexture<'_>,
    ) -> BindGroup {
        let raw = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: pipeline_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.raw.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&albedo.texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&albedo.sampler.raw),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(&shadow.texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::Sampler(&shadow.sampler.raw),
                },
            ],
        });
        BindGroup { raw }
    }

    /// Like [`create_textured_shadow_bind_group`](Self::create_textured_shadow_bind_group)
    /// but without an albedo texture/sampler — the shape an *untextured*
    /// (vertex-colored) shadow-receiving lit shader needs: uniform at
    /// `@binding(0)`, shadow map at `@binding(1)`, shadow sampler at
    /// `@binding(2)`.
    pub fn create_shadow_bind_group(
        &self,
        pipeline_layout: &wgpu::BindGroupLayout,
        buffer: &Buffer,
        shadow_map: &Texture,
        shadow_sampler: &Sampler,
    ) -> BindGroup {
        let raw = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: pipeline_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: buffer.raw.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&shadow_map.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&shadow_sampler.raw),
                },
            ],
        });
        BindGroup { raw }
    }
}
